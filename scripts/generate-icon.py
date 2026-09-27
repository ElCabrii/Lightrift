"""Render the existing Lightrift mark at Windows icon sizes (requires Pillow)."""
from pathlib import Path
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parents[1]
size = 1024
image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
draw = ImageDraw.Draw(image)
draw.rounded_rectangle((16, 16, 1008, 1008), radius=210, fill="#0e1218")
# Same six-point mint mark as the app's navigation rail.
points = [(-12, -16), (13, -16), (3, -3), (10, 16), (-4, 16), (-12, -4)]
draw.polygon([(512 + x * 20, 512 + y * 20) for x, y in points], fill="#6de2c4")
image.resize((256, 256), Image.Resampling.LANCZOS).save(root / "assets/lightrift.png")
image.save(root / "assets/lightrift.ico", sizes=[(n, n) for n in (16, 20, 24, 32, 40, 48, 64, 128, 256)])
