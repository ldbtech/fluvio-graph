import requests, os

API = "https://api.polyhaven.com/assets?type=textures"
assets = requests.get(API).json()
os.makedirs("textures", exist_ok=True)

for name in list(assets.keys()):
    url = (f"https://dl.polyhaven.org/file/ph-assets/"
           f"Textures/jpg/1k/{name}/{name}_diff_1k.jpg")
    try:
        img = requests.get(url, timeout=10)
        if img.status_code == 200:
            with open(f"dataset/textures/{name}.jpg","wb") as f:
                f.write(img.content)
            print(f"Downloaded: {name}")
    except Exception as e:
        print(f"Error downloading {name}: {e}")