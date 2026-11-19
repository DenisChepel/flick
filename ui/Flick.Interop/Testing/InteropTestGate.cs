using System;
using Xunit;

namespace Flick.Interop.Testing
{
    /// <summary>
    /// The one gate for tests that need flick_interop. Source-linked into every test project that
    /// touches natives (excluded from Flick.Interop's own compile, which has no xunit). Locally a
    /// missing library skips the test with the loader's reason; with <c>FLICK_INTEROP_REQUIRED=1</c>
    /// (CI) it fails it, so a packaging mistake can never turn into a quietly green run.
    /// </summary>
    internal static class InteropTestGate
    {
        public static void Require()
        {
            if (InteropLibrary.IsAvailable)
                return;

            var reason = InteropLibrary.UnavailableReason;
            if (Environment.GetEnvironmentVariable(EnvVars.InteropRequired) == "1")
                Assert.Fail($"{EnvVars.InteropRequired}=1 and flick_interop is not available: {reason}");
            Assert.Skip($"flick_interop is not available: {reason}");
        }
    }
}
