import street_fighter_ai_reloaded
import cv2
from time import sleep as pythons_sleepiest_function

x = street_fighter_ai_reloaded.RetroEnvManager("cores/genesis_plus_gx_libretro.so", "roms/Street Fighter II' - Special Champion Edition (USA).zip")

print("Created RetroEnvManager")

x.create_environment("Hello")

print("Created Environment")

while True:
    stuff = x.run()
    print("WOW")
    cv2.imshow("Hello", stuff["Hello"])
    pythons_sleepiest_function(1)