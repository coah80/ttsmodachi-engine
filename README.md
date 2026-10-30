# ttsmodachi-engine

the Tomodachi Life text to speech engine, rewritten in Rust. the audio comes out the same as the game's, byte for byte, and it runs without a 3DS or an emulator.

only US English works for now, other languages come later.

## what is this?

| folder | what it does | made in |
|---|---|---|
| `crates/tts-core` | the engine | Rust |
| `crates/tts-worker` | HTTP server, `POST /render` returns a WAV | Rust |
| `web` | page for typing text, picking a voice and hearing it | HTML, CSS, JS, Vite |
| `tools` | makes the data files the engine needs | Python |

## legal

this project isnt affiliated with Nintendo or Nuance. no game files are in this repo, and you need your own copy of the game to use it. dump it yourself, and dont share your dump or the `data` folder. the MIT license only covers what's written here, it gives you no rights to the game or its data.

## what you need

- Linux, the server doesnt build on Windows or macOS
- Git, Docker, Python 3, [Rust](https://rustup.rs), and Node 22
- your own US copy of Tomodachi Life, updated to Rev 1
- a 3DS with custom firmware, follow [3ds.hacks.guide](https://3ds.hacks.guide) if yours isnt modded

setup uses [ttsmodachi](https://github.com/coah80/ttsmodachi) one time to get the patch and the emulator that make the data files. after that you only need this repo. a Rust setup tool will replace this part later.

## how to run it

### 1. dump the game

1. Open your game in GodMode9 on the 3DS.
   - **cartridge:** open the gamecart, then use `NCSD image options` -> `Decrypt & extract`.
   - **eShop copy:** find the installed title and dump it decrypted.
1. Copy the dump to your PC. `.cxi`, `.3ds`, `.cci`, `.app` and decrypted `.cia` all work.

### 2. patch it

1. Run `git clone https://github.com/coah80/ttsmodachi` and go into that folder.
1. Install [3dstool](https://github.com/dnasdw/3dstool) and [Magikoopa](https://github.com/RicBent/Magikoopa). For a `.cia` you also need [ctrtool](https://github.com/3DSGuy/Project_CTR/releases).
1. Run `python3 tools/patch_rom.py --input /path/to/your/dump.cxi --region US`, with your own dump path.
   - It pauses when the patch is ready to build.
   - If it complains about encryption, dump again with a decrypt option.
1. Open `.rom-patch-work/gamePatch/` in Magikoopa, then press `Make and Insert`.
1. Go back to the tool and let it finish.
   - You should now have `roms/US.cxi`.
1. Run `docker build -t ttsmodachi-worker .` in the same folder.
   - The first build compiles the emulator, so it takes 10 to 30 minutes.

### 3. make the data files

1. Clone this repo with `git clone https://github.com/coah80/ttsmodachi-engine`, then go into that folder.
1. Run this with your own path to `roms`:

   ```sh
   docker run --rm \
     -v /path/to/ttsmodachi/roms:/opt:ro \
     -v "$PWD/data:/data" \
     -v "$PWD/tools:/tools:ro" \
     ttsmodachi-worker python /tools/capture_data.py /data
   ```

   - It takes about a minute and ends with `done, data is in /data`.
   - If it never gets ready or crashes, your dump probably isnt Rev 1.

you only do this once. you can delete the ttsmodachi folder and the docker image after.

### 4. start it

1. Run `cargo run --release -p tts-worker` in the repo root and leave it running.
   - The first build takes a few minutes, then it prints `tts-worker listening on 127.0.0.1:18082`.
   - If it says `Address already in use`, put `RUST_WORKER_PORT=<free port>` in front of this command and the `npm` one.
   - If it says `No such file or directory`, you're not in the repo root or the data files didnt get made.
1. Open a second terminal and go into `web/`.
1. Run `npm install`.
1. Run `npm run dev`, then open the link it prints.
1. Type something and press `generate`.
   - You should hear it, and a `download wav` link shows up.

## calling it without the page

```sh
curl -X POST http://127.0.0.1:18082/render \
  -H 'Content-Type: application/json' \
  -d '{"text":"hello there","mode":"text","voice":{"engine":"tl3ds","lang":"useng","pitch":50,"speed":50,"quality":50,"tone":50,"accent":50,"intonation":1,"volume":100}}' \
  -o hello.wav
```

## license

MIT
