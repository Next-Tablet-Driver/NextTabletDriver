// Minimal .NET console consumer for the NextTabletDriver SDK.
//
// Runs standalone -- no NextTabletDriver desktop app needs to be installed
// or running. Build/run instructions: see README.md in this directory.

using System;
using System.Threading;
using NextTabletDriver.Sdk;

Console.WriteLine($"ntd_sdk ABI version: {NtdClient.AbiVersion}");

using var client = new NtdClient();

// Absolute mode, a small active area in millimeters. Values are clamped to
// the tablet's real physical surface by the engine.
client.SetMode(DriverMode.Absolute);
client.SetActiveArea(0f, 0f, 152f, 95f, 0f);

for (var i = 0; i < 200; i++)
{
    var state = client.PollState();

    // `IsConnected` means "a pen is currently detected", not "a tablet is
    // plugged in" -- a tablet can be open and streaming (status 1 = out of
    // range) with no pen anywhere near its surface. Status 0 means no
    // supported tablet has been found at all.
    string message;
    if (state.IsConnected)
    {
        message = $"u={state.U:F3} v={state.V:F3} pressure={state.Pressure} status={state.Status} device=\"{state.DeviceName}\"";
    }
    else if (state.Status == 0)
    {
        message = "(no tablet detected)";
    }
    else
    {
        message = $"(tablet found, pen out of range, status={state.Status})";
    }

    Console.WriteLine(message);

    Thread.Sleep(50);
}
