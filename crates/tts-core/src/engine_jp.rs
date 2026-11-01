

use crate::jp;
use crate::jp::voice;

pub(crate) use voice::{encode_text, Context, Error};

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_text(
    engine: &mut jp::Engine<'_>,
    tables: &jp::Tables<'_>,
    presets: &[u8],
    text: &str,
    text_workspace: &mut [u8; 0x400],
    parameters: &[(i32, i32)],
    context: &Context,
    teardown: bool,
    pcm: &mut jp::PcmLog<'_>,
) -> Result<(), Error> {
    voice::encode_text(text, text_workspace)?;
    voice::render(
        engine,
        tables,
        presets,
        text_workspace,
        parameters,
        context,
        teardown,
        pcm,
    )
}
