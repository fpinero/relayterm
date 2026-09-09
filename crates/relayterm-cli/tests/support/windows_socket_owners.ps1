param(
    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$source = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class RelaytermSocketOwners
{
    private const uint ErrorInsufficientBuffer = 122;
    private const int TcpTableOwnerPidAll = 5;
    private const int UdpTableOwnerPid = 1;

    [DllImport("iphlpapi.dll", SetLastError = true)]
    private static extern uint GetExtendedTcpTable(
        IntPtr table,
        ref int size,
        bool order,
        int family,
        int tableClass,
        uint reserved);

    [DllImport("iphlpapi.dll", SetLastError = true)]
    private static extern uint GetExtendedUdpTable(
        IntPtr table,
        ref int size,
        bool order,
        int family,
        int tableClass,
        uint reserved);

    private delegate uint ReadTable(IntPtr table, ref int size);

    private static void ReadOwners(HashSet<uint> owners, ReadTable read, int rowSize, int pidOffset)
    {
        int size = 0;
        uint status = read(IntPtr.Zero, ref size);
        if (status != ErrorInsufficientBuffer)
        {
            throw new InvalidOperationException("Network table size query failed.");
        }

        IntPtr table = Marshal.AllocHGlobal(size);
        try
        {
            status = read(table, ref size);
            if (status != 0)
            {
                throw new InvalidOperationException("Network table query failed.");
            }
            int count = Marshal.ReadInt32(table);
            long row = table.ToInt64() + sizeof(uint);
            for (int index = 0; index < count; index++)
            {
                owners.Add(unchecked((uint)Marshal.ReadInt32(new IntPtr(row + pidOffset))));
                row += rowSize;
            }
        }
        finally
        {
            Marshal.FreeHGlobal(table);
        }
    }

    public static uint[] ReadAll()
    {
        const int AfInet = 2;
        const int AfInet6 = 23;
        var owners = new HashSet<uint>();
        ReadOwners(owners, delegate(IntPtr table, ref int size) {
            return GetExtendedTcpTable(table, ref size, false, AfInet, TcpTableOwnerPidAll, 0);
        }, 24, 20);
        ReadOwners(owners, delegate(IntPtr table, ref int size) {
            return GetExtendedTcpTable(table, ref size, false, AfInet6, TcpTableOwnerPidAll, 0);
        }, 56, 52);
        ReadOwners(owners, delegate(IntPtr table, ref int size) {
            return GetExtendedUdpTable(table, ref size, false, AfInet, UdpTableOwnerPid, 0);
        }, 12, 8);
        ReadOwners(owners, delegate(IntPtr table, ref int size) {
            return GetExtendedUdpTable(table, ref size, false, AfInet6, UdpTableOwnerPid, 0);
        }, 28, 24);
        var result = new uint[owners.Count];
        owners.CopyTo(result);
        return result;
    }
}
'@

Add-Type -TypeDefinition $source -Language CSharp
[RelaytermSocketOwners]::ReadAll() | Set-Content -LiteralPath $OutputPath -Encoding ascii
