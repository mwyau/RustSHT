import hashlib
import struct

lanes = 64
basis = []
exponents = []
for lane in range(lanes):
    vector_lane = lane % 4
    vector = lane // 4
    exceptional = vector_lane == 0 or (vector < 7 and vector_lane == 1)
    basis.append(0.375 if exceptional else 0.21 + ((lane * 29 % 101) * 0.0061))
    exponents.append(5 if exceptional else 0)

channels = []
for kind, offsets in (
    (0, (11, 13)),
    (1, (17, 19)),
    (2, (23, 25)),
    (3, (31, 33)),
):
    values = []
    for channel, multiplier in enumerate(offsets):
        if kind == 0:
            values.append([0.13 + (((lane * multiplier + 7) % 97) * 0.004) for lane in range(lanes)])
        elif kind == 1:
            values.append([-0.17 + (((lane * multiplier + 3) % 89) * 0.003) for lane in range(lanes)])
        elif kind == 2:
            values.append([0.09 + (((lane * multiplier + 5) % 83) * 0.005) for lane in range(lanes)])
        else:
            values.append([-0.12 + (((lane * multiplier + 2) % 79) * 0.004) for lane in range(lanes)])
    channels.append(values)

same_real, same_imag, opposite_real, opposite_imag = channels
payload = bytearray()
for channel_arrays in channels:
    for channel in channel_arrays:
        for value in channel:
            payload += struct.pack("<d", value)
for value in basis:
    payload += struct.pack("<d", value)
for value in exponents:
    payload += struct.pack("<q", value)
for _ in range(lanes):
    payload += struct.pack("<d", 1.0)
print(hashlib.sha256(payload).hexdigest())
