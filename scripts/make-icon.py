import math
import os

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DESIGN = 1024
SCALE = 4
SIZE = DESIGN * SCALE


def point(x, y):
    return (x * SCALE, y * SCALE)


def gradient(start, end):
    vertical = Image.linear_gradient("L").resize((SIZE, SIZE))
    horizontal = vertical.rotate(90)
    ramp = ImageChops.add(horizontal, vertical, scale=2)
    base = Image.new("RGB", (SIZE, SIZE), start)
    top = Image.new("RGB", (SIZE, SIZE), end)
    return Image.composite(top, base, ramp)


def hexagon(cx, cy, radius):
    return [point(cx + radius * math.cos(math.radians(angle)), cy + radius * math.sin(math.radians(angle))) for angle in (-90, -30, 30, 90, 150, 210)]


def sparkle(draw, cx, cy, outer, inner, fill):
    points = []
    for index in range(8):
        angle = math.radians(-90 + index * 45)
        radius = outer if index % 2 == 0 else inner
        points.append(point(cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
    draw.polygon(points, fill=fill)


def main():
    canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))

    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rounded_rectangle([point(40, 40), point(984, 984)], radius=210 * SCALE, fill=255)
    background = gradient((38, 193, 217), (9, 52, 80)).convert("RGBA")
    canvas.paste(background, (0, 0), mask)

    glow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse([point(180, 120), point(844, 700)], fill=(255, 255, 255, 46))
    glow = glow.filter(ImageFilter.GaussianBlur(90 * SCALE))
    canvas = Image.alpha_composite(canvas, Image.composite(glow, Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0)), mask))

    cx, cy, radius = 512, 560, 285
    vertices = hexagon(cx, cy, radius)
    center = point(cx, cy)

    shadow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).polygon([(x, y + 44 * SCALE) for x, y in vertices], fill=(2, 20, 36, 120))
    shadow = shadow.filter(ImageFilter.GaussianBlur(34 * SCALE))
    canvas = Image.alpha_composite(canvas, shadow)

    cube = ImageDraw.Draw(canvas)
    top, upper_right, lower_right, bottom, lower_left, upper_left = vertices
    cube.polygon([top, upper_right, center, upper_left], fill=(244, 254, 255, 255))
    cube.polygon([upper_left, center, bottom, lower_left], fill=(150, 226, 242, 255))
    cube.polygon([upper_right, lower_right, bottom, center], fill=(52, 172, 205, 255))

    edge = (10, 56, 84, 255)
    width = 15 * SCALE
    for end in (upper_right, upper_left, bottom):
        cube.line([center, end], fill=edge, width=width)
    for vertex in (upper_right, upper_left, bottom):
        r = width // 2
        cube.ellipse([center[0] - r, center[1] - r, center[0] + r, center[1] + r], fill=edge)
        cube.ellipse([vertex[0] - r, vertex[1] - r, vertex[0] + r, vertex[1] + r], fill=edge)
    outline = vertices + [vertices[0]]
    cube.line(outline, fill=edge, width=width, joint="curve")
    for vertex in vertices:
        r = width // 2
        cube.ellipse([vertex[0] - r, vertex[1] - r, vertex[0] + r, vertex[1] + r], fill=edge)

    sparkle(cube, 790, 258, 92, 24, (255, 255, 255, 255))
    sparkle(cube, 872, 372, 40, 11, (255, 255, 255, 230))

    icon = canvas.resize((DESIGN, DESIGN), Image.LANCZOS)
    icon.save(os.path.join(ROOT, "app-icon.png"))


if __name__ == "__main__":
    main()
