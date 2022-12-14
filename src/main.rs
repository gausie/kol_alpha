use anyhow::{Context, Result};
use gif::{Decoder, Encoder, Frame, Repeat};
use std::borrow::Cow;
use std::fs::File;

fn classify_alphas(palette: &[u8]) -> Result<Vec<u8>> {
    let mut transparents = vec![];
    let mut i = 0;

    while (i * 3) < palette.len() {
        // We're assuming these are greyscale and as such never checking more
        // than the first byte
        if (palette[i * 3]) > 203 {
            transparents.push(
                i.try_into()
                    .context("Palette length too large to fit into u8")?,
            );
        }
        i += 1;
    }

    Ok(transparents)
}

fn eraser(
    index: u16,
    width: u16,
    visited: &mut Vec<u16>,
    input: &Vec<u8>,
    output: &mut Vec<u8>,
    alphas: &Vec<u8>,
) -> Result<()> {
    let length: u16 = input
        .len()
        .try_into()
        .context("Canvas length too large to fit into u16")?;

    if visited.contains(&index) || index >= length {
        return Ok(());
    }

    visited.push(index);

    if !alphas.contains(&input[index as usize]) {
        return Ok(());
    }

    // Set to white
    output[index as usize] = 0;

    let row = index / width;
    let column = index % width;

    // Look to the left if we're not on the left-most column
    if column > 0 {
        eraser(index - 1, width, visited, input, output, alphas)?;
    }

    // Look to the right if we're not on the right-most column
    if column < width - 1 {
        eraser(index + 1, width, visited, input, output, alphas)?;
    }

    // Look above if we're not on the top-most row
    if row > 0 {
        eraser(index - width, width, visited, input, output, alphas)?;
    }

    // Look below if we're not on the bottom-most row
    if row < (length / width) - 1 {
        eraser(index + width, width, visited, input, output, alphas)?;
    }

    Ok(())
}

fn create_encoder(width: u16, height: u16) -> Result<Encoder<File>> {
    let file = File::create("./output.gif").context("Failed to create output file")?;
    let output_palette: [u8; 6] = [255, 255, 255, 0, 0, 0];
    let mut encoder = Encoder::new(file, width, height, &output_palette)
        .context("Failed to create new encoder")?;
    encoder
        .set_repeat(Repeat::Infinite)
        .context("Failed to set gif to repeat infinitely")?;
    Ok(encoder)
}

fn main() -> Result<()> {
    let name = "cassagnome";
    let file = File::open(format!("./src/{}.gif", name)).context("Failed to open input file")?;

    let mut decoder = Decoder::new(file).context("Failed to decode input file")?;

    let width = decoder.width();
    let height = decoder.height();

    // Assess the gif's palette and return a list of indices that we should consider
    // transparent when flood filling.
    let alphas = classify_alphas(
        decoder
            .palette()
            .context("Failed to decode input file palette")?,
    )?;

    let mut encoder = create_encoder(width, height).context("Failed to create encoder")?;

    let canvas_length = width * height;
    let mut canvas: Vec<u8> = vec![0; canvas_length as usize];

    while let Ok(Some(frame)) = decoder.read_next_frame() {
        // Subframe offset in canvas
        let offset = (frame.top * width) + frame.left;
        // Apply new frame to the canvas we're maintaining
        for y in 0..frame.height {
            for x in 0..frame.width {
                let value = frame.buffer[((y * frame.width) + x) as usize];
                let i = (offset + x + (width * y)) as usize;
                // Only change the index if it is not our "transparent" index
                canvas[i] = match frame.transparent {
                    Some(alpha) if alpha == value => canvas[i],
                    None | Some(_) => value,
                };
            }
        }

        // Start with a black canvas - i.e our default is black unless touched by
        // the erase from the starting points
        let mut erased = vec![1; canvas_length as usize];

        let mut visited = vec![];
        // Start a flood erase from each corner
        for start in [0, width - 1, canvas_length - width, canvas_length - 1] {
            eraser(start, width, &mut visited, &canvas, &mut erased, &alphas)?;
        }

        // Extract a subframe from our erased image the same size as the input frame
        let mut new_buffer = vec![];
        for y in 0..frame.height {
            for x in 0..frame.width {
                let i = (offset + x + (width * y)) as usize;
                new_buffer.push(erased[i]);
                // We could consider comparing our erased[i] to our previous canvas here
                // and use the transparent index, but it's hardly worth it.
            }
        }

        let mut new_frame = Frame::clone(frame);
        new_frame.buffer = Cow::Borrowed(&new_buffer);

        if let Err(e) = encoder.write_frame(&new_frame) {
            println!("Frame was skipped as could not be written {:?}", e);
        }
    }

    Ok(())
}
