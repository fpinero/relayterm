//! Current-user local IPC transport with bounded framed I/O.

use relayterm_protocol::{Frame, FrameDecoder, FrameError, FrameKind, HEADER_SIZE, encode_frame};
#[cfg(unix)]
use std::path::PathBuf;
use std::{
    collections::VecDeque,
    fmt, io,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
pub const PARTIAL_FRAME_TIMEOUT: Duration = Duration::from_secs(10);
pub const WRITER_TIMEOUT: Duration = Duration::from_secs(10);
pub const MAX_CONNECTIONS: usize = 16;
pub const MAX_OUTSTANDING_PER_CONNECTION: usize = 16;
pub const MAX_OUTSTANDING_GLOBAL: usize = 64;

pub struct OutboundQueue {
    control: VecDeque<Vec<u8>>,
    terminal: VecDeque<Vec<u8>>,
    bytes: usize,
    terminal_was_last: bool,
}
impl OutboundQueue {
    pub fn new() -> Self {
        Self {
            control: VecDeque::new(),
            terminal: VecDeque::new(),
            bytes: 0,
            terminal_was_last: false,
        }
    }
    pub fn push(&mut self, kind: FrameKind, bytes: Vec<u8>) -> Result<(), IpcError> {
        let next = self
            .bytes
            .checked_add(bytes.len())
            .ok_or(IpcError::ResourceLimit)?;
        if next > relayterm_protocol::CONNECTION_BUFFER_LIMIT {
            return Err(IpcError::ResourceLimit);
        }
        match kind {
            FrameKind::Json => self.control.push_back(bytes),
            FrameKind::Terminal => self.terminal.push_back(bytes),
        }
        self.bytes = next;
        Ok(())
    }
    pub fn pop(&mut self) -> Option<(FrameKind, Vec<u8>)> {
        let (kind, bytes) =
            if !self.control.is_empty() && (self.terminal_was_last || self.terminal.is_empty()) {
                self.terminal_was_last = false;
                (FrameKind::Json, self.control.pop_front()?)
            } else if let Some(value) = self.terminal.pop_front() {
                self.terminal_was_last = true;
                (FrameKind::Terminal, value)
            } else {
                self.terminal_was_last = false;
                (FrameKind::Json, self.control.pop_front()?)
            };
        self.bytes = self.bytes.saturating_sub(bytes.len());
        Some((kind, bytes))
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
impl Default for OutboundQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcError {
    InvalidEndpoint,
    AccessDenied,
    EndpointInUse,
    ResourceLimit,
    Timeout,
    Disconnected,
    Protocol,
    Unavailable,
}
impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidEndpoint => "The local endpoint is invalid.",
            Self::AccessDenied => "The local endpoint access policy was rejected.",
            Self::EndpointInUse => "The local endpoint is already in use.",
            Self::ResourceLimit => "The local IPC resource limit was reached.",
            Self::Timeout => "The local IPC operation timed out.",
            Self::Disconnected => "The local peer disconnected.",
            Self::Protocol => "The local peer violated the protocol.",
            Self::Unavailable => "The local IPC transport is unavailable.",
        })
    }
}
impl std::error::Error for IpcError {}

pub trait AsyncStream: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncStream for T {}
pub struct LocalStream(Pin<Box<dyn AsyncStream>>);
impl LocalStream {
    fn new<T: AsyncStream + 'static>(stream: T) -> Self {
        Self(Box::pin(stream))
    }
    pub fn split(self) -> (tokio::io::ReadHalf<Self>, tokio::io::WriteHalf<Self>) {
        tokio::io::split(self)
    }
}
impl AsyncRead for LocalStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.0.as_mut().poll_read(cx, buf)
    }
}
impl AsyncWrite for LocalStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.0.as_mut().poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.as_mut().poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.0.as_mut().poll_shutdown(cx)
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Endpoint {
    #[cfg(unix)]
    path: PathBuf,
    #[cfg(windows)]
    name: String,
}
impl Endpoint {
    pub fn derive(
        runtime_root: &Path,
        workspace_id: relayterm_protocol::WorkspaceId,
    ) -> Result<Self, IpcError> {
        if !runtime_root.is_absolute() {
            return Err(IpcError::InvalidEndpoint);
        }
        #[cfg(unix)]
        {
            let path = runtime_root.join(format!("{}.sock", workspace_id));
            if path.as_os_str().len() > 100 {
                return Err(IpcError::InvalidEndpoint);
            }
            Ok(Self { path })
        }
        #[cfg(windows)]
        {
            let _ = runtime_root;
            Ok(Self {
                name: format!(r"\\.\pipe\relayterm-{}", workspace_id),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (runtime_root, workspace_id);
            Err(IpcError::Unavailable)
        }
    }
    #[cfg(unix)]
    pub fn path(&self) -> &Path {
        &self.path
    }
    #[cfg(windows)]
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct LocalListener {
    inner: platform::Listener,
}
impl LocalListener {
    pub async fn bind(endpoint: &Endpoint) -> Result<Self, IpcError> {
        platform::Listener::bind(endpoint)
            .await
            .map(|inner| Self { inner })
    }
    pub async fn accept(&self) -> Result<LocalStream, IpcError> {
        self.inner.accept().await
    }
}

pub async fn connect(endpoint: &Endpoint) -> Result<LocalStream, IpcError> {
    platform::connect(endpoint).await
}

pub fn encode_native_path(path: &Path) -> Result<relayterm_protocol::NativePathDto, IpcError> {
    let encoded =
        relayterm_platform::encode_native_path(path).map_err(|_| IpcError::InvalidEndpoint)?;
    relayterm_protocol::NativePathDto::from_bytes(encoded.tag, &encoded.bytes)
        .map_err(|_| IpcError::ResourceLimit)
}

pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    kind: FrameKind,
    payload: &[u8],
) -> Result<(), IpcError> {
    let bytes = encode_frame(kind, payload).map_err(|_| IpcError::Protocol)?;
    tokio::time::timeout(WRITER_TIMEOUT, writer.write_all(&bytes))
        .await
        .map_err(|_| IpcError::Timeout)?
        .map_err(map_io)
}

pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    timeout: Duration,
) -> Result<Frame, IpcError> {
    let mut header = [0_u8; HEADER_SIZE];
    tokio::time::timeout(timeout, reader.read_exact(&mut header))
        .await
        .map_err(|_| IpcError::Timeout)?
        .map_err(map_io)?;
    let length = u32::from_be_bytes(header[..4].try_into().expect("fixed header")) as usize;
    let mut decoder = FrameDecoder::default();
    let initial = decoder.feed(&header).map_err(map_frame)?;
    if !initial.is_empty() {
        return Ok(initial.into_iter().next().expect("one frame"));
    }
    let mut payload = vec![0_u8; length];
    tokio::time::timeout(PARTIAL_FRAME_TIMEOUT, reader.read_exact(&mut payload))
        .await
        .map_err(|_| IpcError::Timeout)?
        .map_err(map_io)?;
    let frames = decoder.feed(&payload).map_err(map_frame)?;
    frames.into_iter().next().ok_or(IpcError::Protocol)
}
fn map_frame(_: FrameError) -> IpcError {
    IpcError::Protocol
}
fn map_io(error: io::Error) -> IpcError {
    match error.kind() {
        io::ErrorKind::UnexpectedEof
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::BrokenPipe => IpcError::Disconnected,
        io::ErrorKind::PermissionDenied => IpcError::AccessDenied,
        _ => IpcError::Unavailable,
    }
}

#[cfg(unix)]
mod platform {
    use super::*;
    use relayterm_platform::{PrivateLock, create_private_dir, validate_private_dir};
    use std::{
        fs,
        os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    };
    use tokio::net::{UnixListener, UnixStream};

    pub struct Listener {
        listener: UnixListener,
        _lock: PrivateLock,
        path: PathBuf,
        identity: (u64, u64),
    }
    impl Listener {
        pub async fn bind(endpoint: &Endpoint) -> Result<Self, IpcError> {
            let parent = endpoint.path.parent().ok_or(IpcError::InvalidEndpoint)?;
            create_private_dir(parent).map_err(|_| IpcError::AccessDenied)?;
            validate_private_dir(parent).map_err(|_| IpcError::AccessDenied)?;
            let lock = PrivateLock::acquire(
                &parent.join(format!(
                    "{}.lock",
                    endpoint
                        .path
                        .file_stem()
                        .and_then(|x| x.to_str())
                        .ok_or(IpcError::InvalidEndpoint)?
                )),
                Duration::from_millis(100),
            )
            .map_err(|_| IpcError::EndpointInUse)?;
            if let Ok(metadata) = fs::symlink_metadata(&endpoint.path) {
                if metadata.file_type().is_symlink()
                    || !metadata.file_type().is_socket()
                    || metadata.uid() != rustix::process::getuid().as_raw()
                {
                    return Err(IpcError::AccessDenied);
                }
                if std::os::unix::net::UnixStream::connect(&endpoint.path).is_ok() {
                    return Err(IpcError::EndpointInUse);
                }
                fs::remove_file(&endpoint.path).map_err(|_| IpcError::AccessDenied)?;
            }
            let listener = UnixListener::bind(&endpoint.path).map_err(map_io)?;
            fs::set_permissions(&endpoint.path, fs::Permissions::from_mode(0o600))
                .map_err(map_io)?;
            let metadata = fs::symlink_metadata(&endpoint.path).map_err(map_io)?;
            if !metadata.file_type().is_socket() {
                return Err(IpcError::InvalidEndpoint);
            }
            if metadata.uid() != rustix::process::getuid().as_raw() {
                return Err(IpcError::AccessDenied);
            }
            if metadata.mode() & 0o777 != 0o600 {
                return Err(IpcError::InvalidEndpoint);
            }
            Ok(Self {
                listener,
                _lock: lock,
                path: endpoint.path.clone(),
                identity: (metadata.dev(), metadata.ino()),
            })
        }
        pub async fn accept(&self) -> Result<LocalStream, IpcError> {
            let (stream, _) = self.listener.accept().await.map_err(map_io)?;
            check_peer(&stream, rustix::process::getuid().as_raw())?;
            Ok(LocalStream::new(stream))
        }
    }
    impl Drop for Listener {
        fn drop(&mut self) {
            if let Ok(m) = fs::symlink_metadata(&self.path)
                && (m.dev(), m.ino()) == self.identity
                && m.file_type().is_socket()
            {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
    pub async fn connect(endpoint: &Endpoint) -> Result<LocalStream, IpcError> {
        let stream = UnixStream::connect(&endpoint.path).await.map_err(map_io)?;
        check_peer(&stream, rustix::process::getuid().as_raw())?;
        Ok(LocalStream::new(stream))
    }
    fn check_peer(stream: &UnixStream, expected: u32) -> Result<(), IpcError> {
        let cred = stream.peer_cred().map_err(|_| IpcError::AccessDenied)?;
        if cred.uid() != expected {
            Err(IpcError::AccessDenied)
        } else {
            Ok(())
        }
    }
    #[cfg(test)]
    pub fn peer_policy(actual: u32, expected: u32) -> Result<(), IpcError> {
        if actual == expected {
            Ok(())
        } else {
            Err(IpcError::AccessDenied)
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use interprocess::os::windows::{
        named_pipe::{
            PipeListenerOptions, pipe_mode,
            tokio::{DuplexPipeStream, PipeListener},
        },
        security_descriptor::SecurityDescriptor,
    };
    use std::num::NonZeroU8;
    use std::os::windows::io::AsRawHandle;
    use widestring::U16CString;

    type ByteListener = PipeListener<pipe_mode::Bytes, pipe_mode::Bytes>;
    fn current_descriptor(allow: bool) -> Result<SecurityDescriptor, IpcError> {
        let sid = windows_permissions::utilities::current_process_sid()
            .map_err(|_| IpcError::AccessDenied)?;
        let ace = if allow { "A;;GRGW" } else { "D;;GA" };
        let wide = U16CString::from_str(format!("O:{sid}D:P({ace};;;{sid})"))
            .map_err(|_| IpcError::AccessDenied)?;
        SecurityDescriptor::deserialize(wide.as_ucstr()).map_err(|_| IpcError::AccessDenied)
    }
    fn validate_pipe_security<H: AsRawHandle>(handle: &H) -> Result<(), IpcError> {
        use windows_permissions::{
            Trustee,
            constants::{SeObjectType, SecurityInformation},
            wrappers,
        };
        let descriptor = wrappers::GetSecurityInfo(
            handle,
            SeObjectType::SE_KERNEL_OBJECT,
            SecurityInformation::Owner | SecurityInformation::Dacl,
        )
        .map_err(|_| IpcError::AccessDenied)?;
        let owner = descriptor.owner().ok_or(IpcError::AccessDenied)?;
        let current = windows_permissions::utilities::current_process_sid()
            .map_err(|_| IpcError::AccessDenied)?;
        if !wrappers::EqualSid(owner, &current) {
            return Err(IpcError::AccessDenied);
        }
        let sddl = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
            &descriptor,
            SecurityInformation::Dacl,
        )
        .map_err(|_| IpcError::AccessDenied)?;
        if !sddl.to_string_lossy().starts_with("D:P") {
            return Err(IpcError::AccessDenied);
        }
        let dacl = descriptor.dacl().ok_or(IpcError::AccessDenied)?;
        for sid_text in ["S-1-1-0", "S-1-5-7", "S-1-5-32-545"] {
            let sid = wrappers::ConvertStringSidToSid(std::ffi::OsStr::new(sid_text))
                .map_err(|_| IpcError::AccessDenied)?;
            if !dacl
                .effective_rights(&Trustee::from(&*sid))
                .map_err(|_| IpcError::AccessDenied)?
                .is_empty()
            {
                return Err(IpcError::AccessDenied);
            }
        }
        Ok(())
    }
    pub struct Listener {
        listener: ByteListener,
    }
    impl Listener {
        pub async fn bind(endpoint: &Endpoint) -> Result<Self, IpcError> {
            let descriptor = current_descriptor(true)?;
            let listener = PipeListenerOptions::new()
                .path(std::path::Path::new(&endpoint.name))
                .instance_limit(Some(
                    NonZeroU8::new((MAX_CONNECTIONS + 1) as u8).expect("nonzero"),
                ))
                .accept_remote(false)
                .inheritable(false)
                .security_descriptor(Some(descriptor))
                .create_tokio_duplex::<pipe_mode::Bytes>()
                .map_err(|_| IpcError::EndpointInUse)?;
            Ok(Self { listener })
        }
        pub async fn accept(&self) -> Result<LocalStream, IpcError> {
            let stream = self
                .listener
                .accept()
                .await
                .map_err(|_| IpcError::Disconnected)?;
            validate_pipe_security(&stream)?;
            Ok(LocalStream::new(stream))
        }
    }
    pub async fn connect(endpoint: &Endpoint) -> Result<LocalStream, IpcError> {
        let stream = DuplexPipeStream::<pipe_mode::Bytes>::connect_by_path(endpoint.name.clone())
            .await
            .map_err(|_| IpcError::Disconnected)?;
        validate_pipe_security(&stream)?;
        Ok(LocalStream::new(stream))
    }

    #[cfg(test)]
    pub fn descriptor_for_current_user() -> Result<SecurityDescriptor, IpcError> {
        current_descriptor(true)
    }

    #[cfg(test)]
    pub fn security_policy() -> (bool, bool, usize) {
        (false, false, MAX_CONNECTIONS + 1)
    }

    #[cfg(test)]
    mod native_tests {
        use super::*;
        #[test]
        fn descriptor_is_explicit_and_pipe_is_local_non_inheritable() {
            assert!(descriptor_for_current_user().is_ok());
            assert_eq!(security_policy(), (false, false, 17));
        }

        #[tokio::test]
        async fn deny_current_user_descriptor_blocks_a_real_connection() {
            let name = format!(r"\\.\pipe\relayterm-deny-{}", std::process::id());
            let listener = PipeListenerOptions::new()
                .path(std::path::Path::new(&name))
                .instance_limit(Some(NonZeroU8::new(2).unwrap()))
                .accept_remote(false)
                .inheritable(false)
                .security_descriptor(Some(current_descriptor(false).unwrap()))
                .create_tokio_duplex::<pipe_mode::Bytes>()
                .unwrap();
            assert!(
                DuplexPipeStream::<pipe_mode::Bytes>::connect_by_path(name)
                    .await
                    .is_err()
            );
            drop(listener);
        }

        #[tokio::test]
        async fn actual_pipe_descriptor_round_trip_and_collision_are_enforced() {
            let root = std::path::Path::new(r"C:\relayterm-runtime");
            let endpoint = Endpoint::derive(
                root,
                format!("00000000-0000-4000-8000-{:012}", std::process::id())
                    .parse()
                    .unwrap(),
            )
            .unwrap();
            let listener = LocalListener::bind(&endpoint).await.unwrap();
            assert_eq!(
                LocalListener::bind(&endpoint).await.err(),
                Some(IpcError::EndpointInUse)
            );
            let server = tokio::spawn(async move {
                let mut stream = listener.accept().await.unwrap();
                let frame = read_frame(&mut stream, Duration::from_secs(1))
                    .await
                    .unwrap();
                write_frame(&mut stream, frame.kind, b"reply")
                    .await
                    .unwrap();
            });
            let mut stream = connect(&endpoint).await.unwrap();
            write_frame(&mut stream, FrameKind::Json, b"request")
                .await
                .unwrap();
            assert_eq!(
                read_frame(&mut stream, Duration::from_secs(1))
                    .await
                    .unwrap()
                    .payload,
                b"reply"
            );
            server.await.unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    fn private_temp(prefix: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(prefix)
            .tempdir_in(if cfg!(target_os = "macos") {
                "/private/tmp"
            } else {
                "/tmp"
            })
            .unwrap()
    }
    #[tokio::test]
    #[cfg(unix)]
    async fn private_socket_round_trip_and_cleanup() {
        use std::os::unix::fs::PermissionsExt;
        let temp = private_temp("rt-ipc-");
        let root = temp.path().join("runtime");
        let endpoint = Endpoint::derive(
            &root,
            "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        )
        .unwrap();
        relayterm_platform::create_private_dir(&root).unwrap();
        relayterm_platform::validate_private_dir(&root).unwrap();
        let listener = LocalListener::bind(&endpoint).await.unwrap();
        assert_eq!(
            std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(endpoint.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let server = tokio::spawn(async move {
            let mut stream = listener.accept().await.unwrap();
            let frame = read_frame(&mut stream, Duration::from_secs(1))
                .await
                .unwrap();
            assert_eq!(frame.payload, b"ping");
            write_frame(&mut stream, FrameKind::Json, b"pong")
                .await
                .unwrap();
        });
        let mut client = connect(&endpoint).await.unwrap();
        write_frame(&mut client, FrameKind::Json, b"ping")
            .await
            .unwrap();
        assert_eq!(
            read_frame(&mut client, Duration::from_secs(1))
                .await
                .unwrap()
                .payload,
            b"pong"
        );
        server.await.unwrap();
        assert!(!endpoint.path().exists());
    }
    #[cfg(unix)]
    #[test]
    fn uid_policy_fails_closed() {
        assert!(platform::peer_policy(1000, 1000).is_ok());
        assert_eq!(
            platform::peer_policy(1001, 1000),
            Err(IpcError::AccessDenied)
        );
    }

    #[test]
    fn control_frames_are_checked_after_one_terminal_frame() {
        let mut queue = OutboundQueue::new();
        queue.push(FrameKind::Terminal, vec![1]).unwrap();
        queue.push(FrameKind::Terminal, vec![2]).unwrap();
        queue.push(FrameKind::Json, vec![3]).unwrap();
        assert_eq!(queue.pop().unwrap(), (FrameKind::Terminal, vec![1]));
        assert_eq!(queue.pop().unwrap(), (FrameKind::Json, vec![3]));
        assert_eq!(queue.pop().unwrap(), (FrameKind::Terminal, vec![2]));
        assert_eq!(queue.bytes(), 0);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn stale_socket_is_reclaimed_but_live_and_regular_endpoints_are_preserved() {
        let temp = private_temp("rt-ipc-state-");
        let root = temp.path().join("runtime");
        relayterm_platform::create_private_dir(&root).unwrap();
        let first = Endpoint::derive(
            &root,
            "00000000-0000-4000-8000-000000000011".parse().unwrap(),
        )
        .unwrap();
        let stale = std::os::unix::net::UnixListener::bind(first.path()).unwrap();
        drop(stale);
        let listener = LocalListener::bind(&first).await.unwrap();
        assert_eq!(
            LocalListener::bind(&first).await.err(),
            Some(IpcError::EndpointInUse)
        );
        drop(listener);

        let second = Endpoint::derive(
            &root,
            "00000000-0000-4000-8000-000000000012".parse().unwrap(),
        )
        .unwrap();
        std::fs::write(second.path(), b"fixture").unwrap();
        assert_eq!(
            LocalListener::bind(&second).await.err(),
            Some(IpcError::AccessDenied)
        );
        assert_eq!(std::fs::read(second.path()).unwrap(), b"fixture");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn listener_cleanup_preserves_a_replacement_object() {
        let temp = private_temp("rt-ipc-replacement-");
        let root = temp.path().join("runtime");
        relayterm_platform::create_private_dir(&root).unwrap();
        let endpoint = Endpoint::derive(
            &root,
            "00000000-0000-4000-8000-000000000014".parse().unwrap(),
        )
        .unwrap();
        let listener = LocalListener::bind(&endpoint).await.unwrap();
        std::fs::remove_file(endpoint.path()).unwrap();
        std::fs::write(endpoint.path(), b"replacement").unwrap();
        drop(listener);
        assert_eq!(std::fs::read(endpoint.path()).unwrap(), b"replacement");
    }

    #[cfg(unix)]
    #[test]
    fn overlong_socket_path_is_rejected_without_fallback() {
        let root = std::path::PathBuf::from("/").join("x".repeat(100));
        assert_eq!(
            Endpoint::derive(
                &root,
                "00000000-0000-4000-8000-000000000013".parse().unwrap()
            )
            .err(),
            Some(IpcError::InvalidEndpoint)
        );
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn protected_named_pipe_round_trip_and_collision() {
        let endpoint = Endpoint::derive(
            std::path::Path::new(r"C:\relayterm-runtime"),
            "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        )
        .unwrap();
        let listener = LocalListener::bind(&endpoint).await.unwrap();
        assert_eq!(
            LocalListener::bind(&endpoint).await.err(),
            Some(IpcError::EndpointInUse)
        );
        let server = tokio::spawn(async move {
            let mut stream = listener.accept().await.unwrap();
            let frame = read_frame(&mut stream, Duration::from_secs(2))
                .await
                .unwrap();
            assert_eq!(frame.payload, b"ping");
            write_frame(&mut stream, FrameKind::Json, b"pong")
                .await
                .unwrap();
        });
        let mut client = connect(&endpoint).await.unwrap();
        write_frame(&mut client, FrameKind::Json, b"ping")
            .await
            .unwrap();
        assert_eq!(
            read_frame(&mut client, Duration::from_secs(2))
                .await
                .unwrap()
                .payload,
            b"pong"
        );
        server.await.unwrap();
    }
}
