# ttsmodachi-engine

the tomodachi life text to speech engine, rewritten in rust. it reads your game dump directly and produces the same audio, byte for byte, for the checked speech cases. us english, eu english, french, german, italian, spanish, korean and japanese boot from their regional dumps.

## what is this?

| folder | what it does | made in |
|---|---|---|
| `crates/tts-core` | the engine | rust |
| `crates/tts-rom` | reads the dump and boots the engine | rust |
| `crates/tts-worker` | http server, `POST /render` returns a wav | rust |
| `web` | page for typing text, picking a voice and hearing it | html, css, js, vite |

## legal

this project isnt affiliated with nintendo or nuance. no game files are included. use your own copy and dont share your dump. the mit license covers this code, not the game or its data.

## voices

| your dump | language | api `lang` | works in this build |
|---|---|---|---|
| us rev 1 | us english | `useng` | yes |
| europe rev 1 | eu english | `eueng` | yes |
| europe rev 1 | french | `fr` | yes |
| europe rev 1 | german | `de` | yes |
| europe rev 1 | italian | `it` | yes |
| europe rev 1 | spanish | `es` | yes |
| korea, original release | korean | `kr` | yes |
| japan rev 2 | japanese | `jp` | yes, checked kana speech |

you only need the dump for the language you use. one eu dump provides all five eu languages. pass more than one regional dump to use their languages together. voices load when first requested. other revisions have not been verified.

## what you need

- linux, git, rust stable and node 22.12 or newer
- your own decrypted game dump for a region and revision in the voices table

## how to run it

### 1. dump the game

1. launch godmode9 on your 3ds by holding `start` while powering on.
   - this needs a console with godmode9 already installed. see its [setup and usage guide](https://3ds.hacks.guide/godmode9-usage).
1. dump your own game to the sd card.
   - **cartridge:** open `[C:] GAMECART`, select `<TitleID>.trim.3ds`, then `NCSD image options...` and `Build CIA from file`.
   - **installed copy:** press `home`, open `Title manager`, select `[A:] SD CARD`, choose tomodachi life, then `Manage Title...` and `Build CIA (standard)`.
   - wait for the dump in `/gm9/out/`. the [dumping guide](https://3ds.hacks.guide/dumping-titles-and-game-cartridges) covers both routes.
1. decrypt the dumped cia with godmode9.
   - select the `.cia`, open `CIA image options...` and choose `Decrypt to 0:/gm9/out`.
   - if you already have a decrypted `.cxi`, `.3ds` or `.cci`, use it directly.
1. copy the decrypted output to your pc, outside this repo.
   - the extension alone doesnt tell you whether it is decrypted. the worker checks when it opens the file.

### 2. get the engine

1. clone the repo:

   ```sh
   git clone https://github.com/coah80/ttsmodachi-engine.git
   ```

   - you should see a new `ttsmodachi-engine` folder. if cloning fails, check the network connection and the error git prints.
1. run `cd ttsmodachi-engine`.

### 3. run it

1. run this with your dump's path:

   ```sh
   TTS_ROM="/path/to/your/dump.cxi" cargo run --release -p tts-worker -- --port 18097
   ```

   - replace `/path/to/your/dump.cxi` with the file you copied from your 3ds.
   - wait for `tts-worker listening on ...`. the first build takes a while.
   - if `cargo` is missing, install rust and reopen the terminal. if a build fails, keep the compiler error when reporting it.
   - if the dump is rejected, find its message in the table below.
   - if port 18097 is in use, change `--port 18097` to a free port. use that same port for the page below.

   to combine regions, repeat `--rom`:

   ```sh
   cargo run --release -p tts-worker -- --port 18097 --rom /path/to/us.cxi --rom /path/to/eu.cxi --rom /path/to/kr.cxi --rom /path/to/jp.cxi
   ```

   `TTS_ROM="/path/to/us.cxi:/path/to/eu.cxi:/path/to/kr.cxi:/path/to/jp.cxi"` also works on linux. repeated `--rom` arguments replace `TTS_ROM`.

### 4. open the page

1. open another terminal in `ttsmodachi-engine/web/`.
1. run `npm ci`.
   - wait for npm to finish. if it reports an unsupported node version, update node to 22.12 or newer.
1. run `RUST_WORKER_PORT=18097 npm run dev`.
   - if you chose another worker port, replace `18097` with it.
1. open the local link vite prints.
   - the language list should contain only the voices your configured dumps provide.
   - if languages fail to load, check that the worker is running and both ports match, then reload the page.
1. choose a language and type some text.
   - changing language or choosing `boy` or `girl` fills in that voice's defaults. you can edit the numbers afterwards.
   - us, eu and korean defaults come from your dump. japanese starts at 50 for all five sliders and intonation 1.
1. press `generate`.
   - you should hear it and see a `download wav` link. press play if the browser blocks autoplay.

the default output gain is 125 percent. voice volume 200 gives the same 250 percent scaling used for the listening checks. us, eu and korean output is mono 16,000 hz; japanese is mono 22,767 hz.

## calling it without the page

```sh
curl -X POST http://127.0.0.1:18097/render \
  -H 'Content-Type: application/json' \
  -d '{"text":"type something here","mode":"text","voice":{"engine":"tl3ds","lang":"useng","pitch":50,"speed":50,"quality":50,"tone":50,"accent":50,"intonation":1,"volume":100}}' \
  -o speech.wav
```

use the port you chose and a `lang` from the voices table. for japanese, set `lang` to `jp` and supply kana text. `GET /languages` lists the available languages and their boy and girl defaults. a language without its dump returns http 400, for example `Language fr needs a decrypted EU Rev 1 dump. Available: useng`.

## dump errors

startup errors begin with `tts-worker:` and exit with code 2.

| message | what to do |
|---|---|
| `set TTS_ROM or pass --rom with your decrypted game dump` | set the variable or add `--rom` with your dump's path |
| `cannot read dump: ...` | check the path and file permissions |
| `encrypted dump: use the decrypted dump option and try again` | dump the game again with decryption enabled |
| `encrypted dump: decrypt the CIA contents before running` | dump a decrypted cia or cxi |
| `wrong revision or modified code: this engine needs US Rev 1` | use a decrypted us rev 1 dump |
| `wrong revision or modified code: Japanese needs JP Rev 2` | use a decrypted japanese rev 2 dump |
| `wrong revision or modified code: EU voices need EU Rev 1` | use a decrypted eu rev 1 dump |
| `wrong revision or modified code: Korean needs the KR original release` | use a decrypted korean original release dump |
| `wrong region or game: use a supported Tomodachi Life US Rev 1, EU Rev 1, KR original release or JP Rev 2 dump` | check the game and region |
| `duplicate region: pass only one US dump`, `duplicate region: pass only one EU dump`, `duplicate region: pass only one KR dump` or `duplicate region: pass only one JP dump` | remove the repeated region from the paths |
| `unsupported dump: use a decrypted .cxi, .3ds, .cci or .cia` | supply the dump itself, not an archive or extracted file |
| `ROM boot could not load voice rules: copy or dump the game again` | copy the dump again or make a fresh decrypted dump |
| `invalid dump: ...` | check that the copy finished and dump the game again if needed |
| `tts-worker: cannot listen on ...` | stop your own worker or choose a free port with `--port` |

## limits

- speech checks cover the listed revisions. other revisions and continuous eu engine-state replay are not covered.
- singing is not supported.
- japanese checks cover captured kana sentences and slider cases. arbitrary japanese text and kanji readings are not covered.
- real 3ds playback and the game's audio mixer are not covered. the wav contains the engine's speech output with the worker's gain applied.
- linux is the tested host. other operating systems are not verified.

## license

mit
