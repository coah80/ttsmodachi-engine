"""Makes the data files the engine runs on, from your own US copy of the game.

Boots the game in citra, warms up the TTS engine, dumps its memory, then stops
the next request right where the engine takes over and dumps that too.

Run inside the ttsmodachi tts-worker image, see the README:
    python /tools/capture_data.py /data
"""
import json
import os
import struct
import sys
import time
from pathlib import Path

os.environ.setdefault("SDL_VIDEODRIVER", "dummy")
os.environ.setdefault("SDL_AUDIODRIVER", "dummy")
os.environ.setdefault("CITRA_LOG_OUTPUT", "discard")
os.environ.setdefault("CITRA_MAX_RUNTIME_SECONDS", "0")

sys.path.insert(0, "/app/api")
import citra  # noqa: E402
import utils  # noqa: E402

citra.CITRA_PORT = utils.findFreeUdpPort()
os.environ.setdefault("CITRA_WORK_DIR", f"/tmp/capture-{citra.CITRA_PORT}")
import tts  # noqa: E402

tts.emu = citra.Citra(port=citra.CITRA_PORT)

REGIONS = [
    ("code", 0x00100000, 0x01000000),
    ("heap", 0x08000000, 0x0C000000),
    ("stack", 0x0FE00000, 0x10000000),
    ("linear", 0x14000000, 0x1C000000),
    ("sysmem", 0x1FF80000, 0x1FF90000),
]
CHUNK = 256 * 1024
PAGE = 4096
DO_TTS = 0x191E40
WIRE_ENTRY = 0x191E4B
BRANCH_TO_SELF = struct.pack("<I", 0xEAFFFFFE)
# engine, magic, request, sp, s16, s17, tick
WIRE_ARGS = [0x14C3FC08, 0x684F424A, 0x08758C60, 0x0E0FFEA0, 0x12345678, 0x87654321, 0x9ABCDEF0]
WARM_TEXT = "Hello, this is a warm up line."
WARM = (50, 50, 50, 50, 50, 1)
PAGES = json.loads((Path(__file__).parent / "template_pages.json").read_text())


def read_exact(emu, addr, size):
    for _ in range(5):
        data = emu.read_memory(addr, size)
        if data is not None and len(data) == size:
            return data
    raise RuntimeError(f"read failed at {addr:#x}")


def dump_region(emu, start, end, path):
    with open(path, "wb") as out:
        for addr in range(start, end, CHUNK):
            out.write(read_exact(emu, addr, min(CHUNK, end - addr)))


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    tts.startEmulator("US", 1)
    emu = tts.emu
    warm = tts.generateText(WARM_TEXT, *WARM, language=1)
    print(f"warm up render: {len(warm)} bytes", flush=True)
    time.sleep(1.5)

    for name, start, end in REGIONS:
        dump_region(emu, start, end, out / f"{name}.bin")
        print(f"dumped {name}", flush=True)
    (out / "warmup.wav").write_bytes(warm)

    original = read_exact(emu, DO_TTS, 4)
    if not emu.write_memory(DO_TTS, BRANCH_TO_SELF):
        raise RuntimeError("could not stop the engine")
    try:
        tts.generateText(WARM_TEXT, *WARM, language=1)
        raise RuntimeError("the render finished, the engine never stopped")
    except TimeoutError:
        pass
    image = {page: read_exact(emu, page, PAGE) for page in PAGES}
    code_page = DO_TTS & ~(PAGE - 1)
    offset = DO_TTS - code_page
    image[code_page] = image[code_page][:offset] + original + image[code_page][offset + 4:]
    with open(out / "reqZ-191e4b.wire", "wb") as f:
        f.write(b"WIR2" + struct.pack("<9I", WIRE_ENTRY, *WIRE_ARGS, len(image)))
        for address, data in sorted(image.items()):
            f.write(struct.pack("<II", address, len(data)))
            f.write(data)
    tts.killEmulator()
    print(f"done, data is in {out}", flush=True)


if __name__ == "__main__":
    main()
