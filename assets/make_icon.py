"""앱 아이콘 생성: icon.ico(exe 파일 아이콘) + icon.rgba(창·작업표시줄 아이콘, 64x64 RGBA 원본 바이트).
실행: python assets/make_icon.py  (Pillow 필요)"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

S = 1024
here = Path(__file__).parent
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# 배경: 둥근 사각형(남색→청록 세로 그라데이션)
grad = Image.new("RGBA", (S, S))
gd = ImageDraw.Draw(grad)
for y in range(S):
    t = y / S
    gd.line([(0, y), (S, y)], fill=(int(30 + 10 * t), int(60 + 110 * t), int(120 + 50 * t), 255))
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([40, 40, S - 40, S - 40], radius=200, fill=255)
img.paste(grad, (0, 0), mask)

# 칩(가운데 사각형 + 양옆 핀)
c0, c1 = 270, S - 270
pin = (235, 245, 255, 255)
for i in range(4):
    p = c0 + 45 + i * (c1 - c0 - 90) / 3
    for a, b in ((c0 - 90, c0), (c1, c1 + 90)):
        d.rounded_rectangle([a, p - 22, b, p + 22], radius=12, fill=pin)  # 좌우 핀
        d.rounded_rectangle([p - 22, a, p + 22, b], radius=12, fill=pin)  # 위아래 핀
d.rounded_rectangle([c0, c0, c1, c1], radius=70, fill=pin)
d.rounded_rectangle([c0 + 40, c0 + 40, c1 - 40, c1 - 40], radius=45, fill=(25, 55, 110, 255))

font = ImageFont.truetype(r"C:\Windows\Fonts\segoeuib.ttf", 260)
d.text((S / 2, S / 2 + 5), "AI", font=font, fill=pin, anchor="mm")

img.resize((256, 256), Image.LANCZOS).save(here / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
img.resize((64, 64), Image.LANCZOS).save(here / "icon.png")
(here / "icon.rgba").write_bytes(img.resize((64, 64), Image.LANCZOS).tobytes())
print("icon.ico, icon.png, icon.rgba")
