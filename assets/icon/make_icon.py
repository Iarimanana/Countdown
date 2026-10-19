"""Draws the Countdown icons.

  countdown.ico / countdown.png / countdown-256.png
      the app: a white stopwatch on teal, with the time left as a mint wedge.
  countdown-setup.ico / countdown-setup-256.png
      the installer: the same stopwatch with a download badge in the corner.

Usage: python3 make_icon.py <output folder>
"""
import sys
from PIL import Image, ImageDraw

S = 1024
TEAL, MINT, WHITE = (13, 122, 110, 255), (159, 227, 214, 255), (255, 255, 255, 255)
ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256]


def app_icon():
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((32, 32, S - 32, S - 32), radius=220, fill=TEAL)

    cx, cy, r = 512, 575, 300
    # Crown and stem on top of the watch.
    d.rounded_rectangle((432, 150, 592, 230), radius=36, fill=WHITE)
    d.rectangle((482, 220, 542, 290), fill=WHITE)
    # Watch face.
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=WHITE)
    inner = r - 62
    d.ellipse((cx - inner, cy - inner, cx + inner, cy + inner), fill=TEAL)
    # Time left: three quarters, counted clockwise from twelve o'clock.
    wedge = inner - 34
    d.pieslice((cx - wedge, cy - wedge, cx + wedge, cy + wedge), start=-90, end=180, fill=MINT)
    # Centre pin.
    d.ellipse((cx - 34, cy - 34, cx + 34, cy + 34), fill=WHITE)
    return img


def setup_icon():
    img = app_icon()
    d = ImageDraw.Draw(img)
    # A white disc in the bottom-right corner, ringed in teal so it stays
    # separate from the watch, holding a teal "download" arrow over a tray.
    # Kept inside the tile (which ends at 992) with room to spare.
    bx, by, br = 738, 738, 196
    d.ellipse((bx - br - 26, by - br - 26, bx + br + 26, by + br + 26), fill=TEAL)
    d.ellipse((bx - br, by - br, bx + br, by + br), fill=WHITE)
    d.rounded_rectangle((bx - 34, by - 128, bx + 34, by + 14), radius=18, fill=TEAL)  # shaft
    d.polygon([(bx - 100, by - 8), (bx + 100, by - 8), (bx, by + 88)], fill=TEAL)  # head
    d.rounded_rectangle((bx - 115, by + 106, bx + 115, by + 146), radius=20, fill=TEAL)  # tray
    return img


def save(img, out, name, with_full_png=False):
    small = img.resize((256, 256), Image.LANCZOS)
    small.save(f"{out}/{name}.ico", sizes=[(s, s) for s in ICO_SIZES])
    small.save(f"{out}/{name}-256.png")
    if with_full_png:
        img.save(f"{out}/{name}.png")


if __name__ == "__main__":
    out = sys.argv[1]
    save(app_icon(), out, "countdown", with_full_png=True)
    save(setup_icon(), out, "countdown-setup")
