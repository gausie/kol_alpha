use gif::{Decoder, Encoder, Repeat};
use std::fs::File;
use std::borrow::Cow;

fn classify_alphas(palette: &[u8]) -> Vec<u8> {
    let mut transparents: Vec<u8> = Vec::new();
    let mut i = 0;

    while (i * 3) < palette.len() {
        // We're assuming these are greyscale and as such never checking more
        // than the first byte
        if (palette[i * 3]) > 203 {
            transparents.push(i.try_into().unwrap());
        }
        i += 1;
    }

    return transparents;
}

fn eraser(
    index: &usize,
    width: usize,
    visited: &mut Vec<usize>,
    input: &Vec<u8>,
    output: &mut Vec<u8>,
    alphas: &Vec<u8>,
) {
    if visited.contains(index) || index < &0 || index >= &input.len() {
        return;
    }

    visited.push(*index);

    if !alphas.contains(&input[*index]) {
        return;
    }

    // Set to white
    output[*index] = 0;

    // Look to the left if we're not on the left-most column
    if index > &(index / width) {
        eraser(&(index - 1), width, visited, input, output, alphas);
    }

    // Look to the right if we're not on the right-most column
    if index < &((index / width) + (width - 1)) {
        eraser(&(index + 1), width, visited, input, output, alphas);
    }

    // Look above if we're not at the top
    if index > &width {
        eraser(&(index - width), width, visited, input, output, alphas);
    }

    // Look below if we're not at the bottom
    if index < &(input.len() - width) {
        eraser(&(index + width), width, visited, input, output, alphas);
    }
}

fn main() {
    let name = "cassagnome";
    let file = File::open("./src/".to_owned() + name + ".gif").unwrap();
    let mut decoder = Decoder::new(file).unwrap();

    let width = usize::from(decoder.width());
    let height = usize::from(decoder.height());

    // Assess the gif's palette and return a list of indices that we should consider
    // transparent when flood filling.
    let alphas = classify_alphas(decoder.palette().unwrap());

    let mut mask = File::create("src/".to_owned() + name + ".mask.gif").unwrap();

    let output_palette: [u8; 6] = [255, 255, 255, 0, 0, 0];
    let mut encoder = Encoder::new(
        &mut mask,
        width.try_into().unwrap(),
        height.try_into().unwrap(),
        &output_palette,
    )
    .unwrap();
    encoder.set_repeat(Repeat::Infinite).unwrap();

    let mut canvas: Vec<u8> = (0..(width * height)).map(|_| 0).collect();
    let black_canvas: Vec<u8> = (0..(width * height)).map(|_| 1).collect();

    while let Some(frame) = decoder.read_next_frame().unwrap() {
        let frame_width = usize::from(frame.width);
        let frame_height = usize::from(frame.height);
        let frame_top = usize::from(frame.top);
        let frame_left = usize::from(frame.left);

        // Subframe offset in canvas
        let offset = (frame_top * width) + frame_left;
        
        // Apply new frame to the canvas we're maintaining
        for y in 0..frame_height {
            for x in 0..frame_width {
                let value = frame.buffer[(y * frame_width) + x];
                let i = offset + x + (width * y);
                // Only change the index if it is not our "transparent" index
                canvas[i] = match frame.transparent {
                    Some(alpha) if alpha == value => canvas[i],
                    None | Some(_) => value,
                };
            }
        }

        // Start with a black canvas - i.e our default is black unless touched by
        // the erase from the starting points
        let mut erased = black_canvas.clone();

        let mut visited = vec![];
        
        // Start a flood erase from each corner
        for start in [0, width - 1, canvas.len() - width, canvas.len() - 1] {
            eraser(
                &start,
                width,
                &mut visited,
                &canvas,
                &mut erased,
                &alphas,
            );
        }

        // Extract a subframe from our erased image the same size as the input frame
        let mut new_buffer = vec![];
        for y in 0..frame_height {
            for x in 0..frame_width {
                let i = offset + x + (width * y);
                new_buffer.push(erased[i]);
                // We could consider comparing our erased[i] to our previous canvas here
                // and use the transparent index, but it's hardly worth it.
            }
        }

        let mut new_frame = frame.clone();
        new_frame.buffer = Cow::Borrowed(&new_buffer);
        encoder.write_frame(&new_frame).unwrap();
    }
}