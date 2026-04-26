from PIL import Image
import os

def generate_icons():
    logo_path = "assets/Rustify-logo.png"
    if not os.path.exists(logo_path):
        print(f"Error: {logo_path} not found")
        return

    img = Image.open(logo_path)
    
    # Ensure transparency is handled
    if img.mode != 'RGBA':
        img = img.convert('RGBA')

    # Tauri icons
    tauri_icons_dir = "desktop/icons"
    os.makedirs(tauri_icons_dir, exist_ok=True)

    # PNG sizes
    sizes = {
        "32x32.png": (32, 32),
        "128x128.png": (128, 128),
        "icon.png": (512, 512)
    }

    for name, size in sizes.items():
        resized = img.resize(size, Image.Resampling.LANCZOS)
        resized.save(os.path.join(tauri_icons_dir, name))
        print(f"Generated {os.path.join(tauri_icons_dir, name)}")

    # ICO file for Windows
    ico_path = os.path.join(tauri_icons_dir, "icon.ico")
    ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    img.save(ico_path, format='ICO', sizes=ico_sizes)
    print(f"Generated {ico_path}")

    # Web Favicon
    dist_dir = "dist"
    if os.path.exists(dist_dir):
        favicon_path = os.path.join(dist_dir, "favicon.ico")
        img.save(favicon_path, format='ICO', sizes=[(32, 32), (48, 48)])
        print(f"Generated {favicon_path}")

if __name__ == "__main__":
    generate_icons()
