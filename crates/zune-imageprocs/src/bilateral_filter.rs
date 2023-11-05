use nanorand::Rng;
use zune_core::bit_depth::BitType;
use zune_image::channel::Channel;
use zune_image::errors::ImageErrors;
use zune_image::image::Image;
use zune_image::traits::OperationsTrait;

use crate::pad::{pad, PadMethod};
use crate::spatial::spatial;
use crate::traits::NumOps;

pub struct BilateralFilter {
    d: i32,
    sigma_color: f32,
    sigma_space: f32,
}

impl BilateralFilter {
    pub fn new(d: i32, sigma_color: f32, sigma_space: f32) -> BilateralFilter {
        BilateralFilter {
            d,
            sigma_color,
            sigma_space,
        }
    }
}

impl OperationsTrait for BilateralFilter {
    fn name(&self) -> &'static str {
        "Bilateral Filter"
    }

    fn execute_impl(&self, image: &mut Image) -> Result<(), ImageErrors> {
        let depth = image.depth();
        let (w, h) = image.dimensions();

        // initialize bilateral coefficients outside of the main loop
        let coeffs = init_bilateral(
            self.d,
            self.sigma_color,
            self.sigma_space,
            usize::from(depth.max_value()) + 1,
        );

        #[cfg(feature = "threads")]
        {
            std::thread::scope(|s| {
                let mut t_results = vec![];
                for channel in image.channels_mut(true) {
                    let result = s.spawn(|| {
                        let mut new_channel = Channel::new_with_bit_type(channel.len(), depth.bit_type());
                        match depth.bit_type() {
                            BitType::U8 => bilateral_filter_int::<u8>(
                                channel.reinterpret_as()?,
                                new_channel.reinterpret_as_mut()?,
                                w,
                                h,
                                &coeffs,
                            ),
                            BitType::U16 => bilateral_filter_int::<u16>(
                                channel.reinterpret_as()?,
                                new_channel.reinterpret_as_mut()?,
                                w,
                                h,
                                &coeffs,
                            ),

                            d => {
                                return Err(ImageErrors::ImageOperationNotImplemented(
                                    self.name(),
                                    d,
                                ));
                            }
                        }
                        *channel = new_channel;
                        Ok(())
                    });
                    t_results.push(result);
                }

                t_results
                    .into_iter()
                    .map(|x| x.join().unwrap())
                    .collect::<Result<Vec<()>, ImageErrors>>()
            })?;
        }

        #[cfg(not(feature = "threads"))]
        {
            for channel in image.channels_mut(true) {
                let mut new_channel = Channel::new_with_bit_type(channel.len(), depth.bit_type());
                match depth.bit_type() {
                    BitType::U8 => {
                        bilateral_filter_int::<u8>(
                            channel.reinterpret_as()?,
                            new_channel.reinterpret_as_mut()?,
                            w,
                            h,
                            &coeffs,
                        );
                    }
                    BitType::U16 => {
                        bilateral_filter_int::<u16>(
                            channel.reinterpret_as()?,
                            new_channel.reinterpret_as_mut()?,
                            w,
                            h,
                            &coeffs,
                        );
                    }

                    d => {
                        return Err(ImageErrors::ImageOperationNotImplemented(self.name(), d));
                    }
                }
                // overwrite with the filtered channel
                *channel = new_channel;
            }
        }

        Ok(())
    }

    fn supported_types(&self) -> &'static [BitType] {
        &[BitType::U8, BitType::U16]
    }
}

pub struct BilateralCoeffs {
    color_weight: Vec<f32>,
    space_weight: Vec<f32>,
    radius: usize,
    makx: usize,
}

fn init_bilateral(
    d: i32, sigma_color: f32, mut sigma_space: f32, color_range: usize,
) -> BilateralCoeffs {
    let gauss_color_coeff = -0.5 / (sigma_color * sigma_color);
    let gauss_space_coeff = -0.5 / (sigma_space * sigma_space);
    let cn = 1;
    let radius: i32;

    // if sigma_color <= 0.0 {
    //     sigma_color = 1.0;
    // }
    if sigma_space <= 0.0 {
        sigma_space = 1.0;
    }

    if d <= 0 {
        radius = (sigma_space * 1.5).round() as _;
    } else {
        radius = d / 2;
    }

    let mut color_weight = vec![0.0_f32; cn * color_range];
    let mut space_weight = vec![0.0_f32; (d * d) as usize];

    // initialize color-related bilateral filter coeffs
    for i in 0..color_range {
        let c = i as f32;
        color_weight[i] = (c * c * gauss_color_coeff).exp();
    }
    let mut makx = 0;
    // initialize space-related bilateral coeffs
    for i in -radius..=radius {
        for j in -radius..=radius {
            let r = (((i * i) + (j * j)) as f32).sqrt();
            if r > radius as f32 {
                continue;
            }
            space_weight[makx] = (r * r * gauss_space_coeff).exp();
            makx += 1;
        }
    }
    return BilateralCoeffs {
        color_weight,
        space_weight,
        radius: radius as usize,
        makx,
    };
}

pub fn bilateral_filter_int<T>(
    src: &[T], dest: &mut [T], width: usize, height: usize, coeffs: &BilateralCoeffs,
) where
    T: Copy + NumOps<T> + Default,
    i32: std::convert::From<T>
{
    let radius = coeffs.radius;

    //pad here
    let padded_input = pad(src, width, height, radius, radius, PadMethod::Replicate);

    let mid = (radius + 1) / 2;

    // use an inner lambda to implement the bilateral loop as it allows us to borrow
    // surrounding variables

    // Carry out the bilateral filter on a single pixel
    // the mid of the area is considered to be the main pixel, the others
    // are it's surrounding.
    //
    // This impl matches opencv bilateral_filter's inner loop, with less pointer chasing as
    // the spatial function sends the right thing to us
    let bilateral_func = |area: &[T]| -> T {
        let mut sum = 0.0;
        let mut wsum = 0.0;
        let val0 = i32::from(area[mid]);

        for (val, space_w) in area
            .iter()
            .zip(coeffs.space_weight.iter())
            .take(coeffs.makx)
        {
            let val = i32::from(*val);
            let abs_val = (val - val0).abs() as usize;

            let w = space_w * coeffs.color_weight[abs_val];
            sum += (val as f32) * w;
            wsum += w;
        }
        return T::from_f32((sum / wsum).round());
    };

    spatial(&padded_input, dest, radius, width, height, bilateral_func);
}

/// Tests to see that the filter can run on supported bit depths
#[test]
fn test_bilateral_simple() {
    use zune_core::colorspace::ColorSpace;

    let w = 100;
    let h = 100;
    let color = ColorSpace::Luma;

    // fill with random items
    let mut input = vec![0_u8; w * h * color.num_components()];
    nanorand::WyRand::new().fill(&mut input);

    let pixels = Image::from_u8(&input, w, h, color);
    let filter = BilateralFilter::new(20, 75.0, 75.0);
    for d in filter.supported_types() {
        let mut c = pixels.clone();
        c.convert_depth(d.to_depth()).unwrap();
        filter.execute(&mut c).unwrap();
    }

}
