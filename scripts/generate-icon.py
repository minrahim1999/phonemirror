#!/usr/bin/env python3
"""Generate AppIcon.icns from the same phone-mirror pixel art used in main.rs."""
import struct
import os

def generate_icon_rgba(size=1024):
    """Generate RGBA pixel data matching the load_icon() in main.rs, scaled up."""
    rgba = bytearray(size * size * 4)

    # Scale factors from the 64x64 original coords to target size
    s = size / 64.0

    phone_left, phone_right = 18.0 * s, 46.0 * s
    phone_top, phone_bottom = 6.0 * s, 56.0 * s
    screen_left, screen_right = 21.0 * s, 43.0 * s
    screen_top, screen_bottom = 12.0 * s, 50.0 * s

    for y in range(size):
        for x in range(size):
            fx, fy = float(x), float(y)

            in_phone = phone_left <= fx <= phone_right and phone_top <= fy <= phone_bottom

            in_screen = screen_left <= fx <= screen_right and screen_top <= fy <= screen_bottom

            # Reflection diagonal lines
            in_reflection = (in_screen
                and fy > 22.0 * s and fy < 40.0 * s
                and fx > 26.0 * s and fx < 40.0 * s
                and (abs(fy - fx + 10.0 * s) < 2.5 * s
                     or abs(fy + fx - 72.0 * s) < 2.5 * s))

            # Home button
            cx, cy = 32.0 * s, 53.0 * s
            in_home = (fx - cx) ** 2 + (fy - cy) ** 2 < 6.0 * s * s

            if in_reflection:
                r, g, b, a = 100, 180, 255, 230
            elif in_screen:
                r, g, b, a = 15, 18, 30, 255
            elif in_home:
                r, g, b, a = 60, 70, 100, 255
            elif in_phone:
                r, g, b, a = 40, 50, 70, 255
            else:
                r, g, b, a = 0, 0, 0, 0

            offset = (y * size + x) * 4
            rgba[offset] = r
            rgba[offset + 1] = g
            rgba[offset + 2] = b
            rgba[offset + 3] = a

    return bytes(rgba)


def make_png(size, rgba_data):
    """Create a PNG file in memory (simple RGBA PNG)."""
    import zlib

    def make_chunk(chunk_type, data):
        chunk = chunk_type + data
        crc = struct.pack('>I', zlib.crc32(chunk) & 0xFFFFFFFF)
        return struct.pack('>I', len(data)) + chunk + crc

    # PNG signature
    sig = b'\x89PNG\r\n\x1a\n'

    # IHDR
    ihdr_data = struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)  # 8bit RGBA
    ihdr = make_chunk(b'IHDR', ihdr_data)

    # IDAT — each row has filter byte 0 + raw RGBA
    raw = b''
    for y in range(size):
        raw += b'\x00'  # filter: None
        row_start = y * size * 4
        raw += rgba_data[row_start:row_start + size * 4]

    idat = make_chunk(b'IDAT', zlib.compress(raw, 9))

    # IEND
    iend = make_chunk(b'IEND', b'')

    return sig + ihdr + idat + iend


def make_icns(iconset_dir, output_path):
    """Build .icns from an iconset directory using iconutil (macOS only)."""
    import subprocess
    result = subprocess.run(
        ['iconutil', '-c', 'icns', '-o', output_path, iconset_dir],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        raise RuntimeError(f"iconutil failed: {result.stderr}")
    print(f"✅ Generated {output_path}")


def main():
    import tempfile

    # Generate sizes needed for .iconset
    sizes = [16, 32, 64, 128, 256, 512, 1024]
    iconset_sizes = {
        16:  [('icon_16x16.png', 16)],
        32:  [('icon_16x16@2x.png', 32), ('icon_32x32.png', 32)],
        64:  [('icon_32x32@2x.png', 64)],
        128: [('icon_128x128.png', 128)],
        256: [('icon_128x128@2x.png', 256), ('icon_256x256.png', 256)],
        512: [('icon_256x256@2x.png', 512), ('icon_512x512.png', 512)],
        1024: [('icon_512x512@2x.png', 1024)],
    }

    # Pre-generate RGBA for each resolution
    rgba_cache = {}
    for sz in sizes:
        print(f"  Generating {sz}x{sz} pixel art...")
        rgba_cache[sz] = generate_icon_rgba(sz)

    # Create iconset directory
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_dir = os.path.dirname(script_dir)
    iconset_dir = os.path.join(project_dir, 'PhoneMirror.iconset')
    os.makedirs(iconset_dir, exist_ok=True)

    for sz, filenames in iconset_sizes.items():
        rgba = rgba_cache[sz]
        png_data = make_png(sz, rgba)
        for fname, _ in filenames:
            path = os.path.join(iconset_dir, fname)
            with open(path, 'wb') as f:
                f.write(png_data)
            print(f"  Wrote {fname}")

    # Convert to .icns
    output_path = os.path.join(project_dir, 'AppIcon.icns')
    make_icns(iconset_dir, output_path)

    # Clean up iconset
    import shutil
    shutil.rmtree(iconset_dir)
    print("🧹 Cleaned up .iconset temp dir")


if __name__ == '__main__':
    main()