namespace Flick.Interop
{
    /// <summary>
    /// The share-region rectangle rules (<c>flick_share_region_*</c>) over
    /// <c>flick_shared::share_region</c>, the one implementation the region helper applies to
    /// every region it mirrors. Rects are in capture space (physical pixels in virtual-desktop
    /// coordinates on Windows, CoreGraphics points on macOS) as (x, y, width, height). Every member:
    /// any thread.
    /// </summary>
    public static unsafe class ShareRegion
    {
        /// <summary>The region the helper would mirror: each side floored at the minimum and then
        /// rounded down to even, the origin untouched. Idempotent.</summary>
        public static (int X, int Y, int Width, int Height) Normalize(int x, int y, int width, int height)
        {
            InteropLibrary.Require();
            var rect = new FlickRect { x = x, y = y, width = width, height = height };
            FlickRect result;
            Native.Check(NativeMethods.flick_share_region_normalize(&rect, &result), "flick_share_region_normalize");
            return (result.x, result.y, result.width, result.height);
        }

        /// <summary>The region after dragging <paramref name="handle"/> by (<paramref name="dx"/>,
        /// <paramref name="dy"/>) from the press-time <paramref name="x"/>, <paramref name="y"/>,
        /// <paramref name="width"/>, <paramref name="height"/>: the body moves it unchanged in size,
        /// any other handle moves only its own edges, keeps the anchored edge pinned across a
        /// crossing, and normalizes the result.</summary>
        public static (int X, int Y, int Width, int Height) ApplyDrag(int x, int y, int width, int height, ShareRegionHandle handle, int dx, int dy)
        {
            InteropLibrary.Require();
            var anchor = new FlickRect { x = x, y = y, width = width, height = height };
            FlickRect result;
            Native.Check(NativeMethods.flick_share_region_apply_drag(&anchor, (int)handle, dx, dy, &result), "flick_share_region_apply_drag");
            return (result.x, result.y, result.width, result.height);
        }
    }
}
