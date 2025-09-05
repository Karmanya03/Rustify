from PIL import Image
import os

# Create a simple 32x32 icon
img = Image.new('RGBA', (32, 32), (0, 0, 255, 255))  # Blue square
ico_path = "desktop/icons/icon.ico"
os.makedirs(os.path.dirname(ico_path), exist_ok=True)
img.save(ico_path, format='ICO', sizes=[(32, 32)])
print(f"Created {ico_path}")
