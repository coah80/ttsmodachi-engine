# ttsmodachi-engine

the Tomodachi Life text to speech engine, ported to Rust, so it runs without a 3DS or an emulator.

## what is this?

- `crates/tts-core`: the engine, a port of the game's own TTS code. It gives the same audio as the game running in an emulator, byte for byte.
- `crates/tts-worker`: an HTTP server around the engine, `POST /render` returns a WAV.
- `web`: a small page for typing text, picking voice settings and hearing the result.
- `tools/capture_data.py`: makes the data files the engine needs, from your own copy of the game.

the engine is Rust, the page is plain HTML, CSS and JS on Vite, the capture tool is Python.

only US English works for now, other languages come later.

no game files are in this repo. the engine needs data from your own legally dumped US copy of the game.

## how to run it

you need Rust, Node, Docker, and your US game dump.

1. Clone [ttsmodachi](https://github.com/coah80/ttsmodachi) and follow its `patch your ROM` section to get `roms/US.cxi`.
1. Run `docker build -t ttsmodachi-worker .` in that clone. The first build compiles Citra, so it takes 10 to 30 minutes.
1. Run this from the root of this repo, with your own path to `roms`. It only needs doing once, and writes to `data/`:

   ```sh
   docker run --rm \
     -v /path/to/ttsmodachi/roms:/opt:ro \
     -v "$PWD/data:/data" \
     -v "$PWD/tools:/tools:ro" \
     ttsmodachi-worker python /tools/capture_data.py /data
   ```

1. Run `cargo run --release -p tts-worker` to start the server on port 18082.
1. Run `npm install && npm run dev` in `web/`, then open the link it prints.

if 18082 is taken, set `RUST_WORKER_PORT` to another port for both the server and the page.

## license

MIT
