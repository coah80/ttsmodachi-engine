# ttsmodachi-engine

the tomodachi life text to speech engine, rewritten in rust. it reads your game dump directly and produces the same audio, byte for byte. only us english works for now.

## what is this?

| folder | what it does | made in |
|---|---|---|
| `crates/tts-core` | the engine | rust |
| `crates/tts-rom` | reads the dump and boots the engine | rust |
| `crates/tts-worker` | http server, `POST /render` returns a wav | rust |
| `web` | page for typing text, picking a voice and hearing it | html, css, js, vite |

## legal

this project isnt affiliated with nintendo or nuance. no game files are included. use your own copy and dont share your dump. the mit license covers this code, not the game or its data.

## what you need

- linux
- rust stable and node 22.12 or newer
- your own decrypted us rev 1 dump of tomodachi life

## how to run it

### 1. dump the game

1. dump your own us rev 1 copy with godmode9's decrypt option.
1. copy the `.cxi`, `.3ds`, `.cci` or `.cia` to your pc. keep it outside this repo.
   - the extension alone doesnt tell you whether it is decrypted. the worker checks when it opens the file.

### 2. run it

1. open a terminal in this repo and run this with your dump's path:

   ```sh
   TTS_ROM="/path/to/your/dump.cxi" cargo run --release -p tts-worker
   ```

   - wait for `tts-worker listening on ...`. the first build takes a while.
   - `encrypted dump` means you need to dump it again with decryption enabled.
   - `wrong region` means you need the us release. `wrong revision or modified code` means it didnt match rev 1.
   - `cannot read dump` means the path or file permissions need fixing.
   - if the port is in use, add `-- --port 18097` to the command. use that same port for the page below.

### 3. open the page

1. open another terminal in `web/`.
1. run `npm ci`, then `npm run dev`.
   - if you chose another worker port, run `RUST_WORKER_PORT=18097 npm run dev` instead.
1. open the local link vite prints, type something and press `generate`.
   - you should hear it and see a `download wav` link.
   - if the request cannot connect, check that the worker is still running and both ports match.

the default output gain is 125 percent. voice volume 200 gives the same 250 percent scaling used for the listening checks.

## calling it without the page

```sh
curl -X POST http://127.0.0.1:18082/render \
  -H 'Content-Type: application/json' \
  -d '{"text":"hello there","mode":"text","voice":{"engine":"tl3ds","lang":"useng","pitch":50,"speed":50,"quality":50,"tone":50,"accent":50,"intonation":1,"volume":100}}' \
  -o hello.wav
```

`--rom /path/to/your/dump.cxi` also works instead of `TTS_ROM`.

## license

mit
