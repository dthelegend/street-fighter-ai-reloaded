import street_fighter_ai_reloaded
import cv2
from time import sleep as pythons_sleepiest_function

x = street_fighter_ai_reloaded.RetroEnvManager("cores/genesis_plus_gx_libretro.so", "roms/sf2/sf2.md")

print("Created RetroEnvManager")

while True:
    stuff = x.step()
    print("WOW")
    cv2.imshow("Hello", stuff)

print("OOF")