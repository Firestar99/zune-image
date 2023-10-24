/*
 * Copyright (c) 2023.
 *
 * This software is free software;
 *
 * You can redistribute it or modify it under terms of the MIT, Apache License or Zlib license
 */
mod numpy_bindings;

use std::fs::read;

use numpy::PyUntypedArray;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use zune_core::bit_depth::BitType;
use zune_image::core_filters::colorspace::ColorspaceConv;
use zune_image::core_filters::depth::Depth;
use zune_image::image::Image;
use zune_image::traits::OperationsTrait;
use zune_imageprocs::auto_orient::AutoOrient;
use zune_imageprocs::box_blur::BoxBlur;
use zune_imageprocs::crop::Crop;
use zune_imageprocs::exposure::Exposure;
use zune_imageprocs::flip::Flip;
use zune_imageprocs::flop::Flop;
use zune_imageprocs::gamma::Gamma;
use zune_imageprocs::gaussian_blur::GaussianBlur;
use zune_imageprocs::invert::Invert;
use zune_imageprocs::scharr::Scharr;
use zune_imageprocs::sobel::Sobel;
use zune_imageprocs::stretch_contrast::StretchContrast;
use zune_imageprocs::threshold::Threshold;
use zune_imageprocs::transpose::Transpose;
use zune_png::zune_core::options::DecoderOptions;

use crate::py_enums::{
    ZImageColorSpace, ZImageDepth, ZImageErrors, ZImageFormats, ZImageThresholdType
};
/// Execute a single filter on an image
///
/// This executes anything that implements OperationsTrait, returning an error if the
/// operation returned an error or okay if operation was successful

fn exec_filter<T: OperationsTrait>(
    img: &mut ZImage, filter: T, in_place: bool
) -> PyResult<Option<ZImage>> {
    let exec = |image: &mut ZImage| -> PyResult<()> {
        if let Err(e) = filter.execute(&mut image.image) {
            return Err(PyErr::new::<PyException, _>(format!(
                "Error converting: {:?}",
                e
            )));
        }
        Ok(())
    };
    if in_place {
        exec(img)?;
        Ok(None)
    } else {
        let mut im_clone = img.clone();
        exec(&mut im_clone)?;

        Ok(Some(im_clone))
    }
}

/// The image class.
#[pyclass]
#[derive(Clone)]
pub struct ZImage {
    image: Image
}

impl ZImage {
    pub(crate) fn new(image: Image) -> ZImage {
        return ZImage { image };
    }
}

#[pymethods]
impl ZImage {
    /// Get the image dimensions as a tuple of width and height
    ///
    /// # Returns
    /// -  A tuple in the format `(width,height)`
    pub fn dimensions(&self) -> (usize, usize) {
        self.image.get_dimensions()
    }
    /// Get the image width
    ///
    /// # Returns
    /// Image width
    pub fn width(&self) -> usize {
        self.image.get_dimensions().0
    }
    /// Get the image height
    ///
    /// # Returns
    /// Image height
    pub fn height(&self) -> usize {
        self.image.get_dimensions().1
    }
    /// Get the image colorspace
    ///
    /// # Returns
    /// - The current image colorspace
    ///
    /// # See
    /// - [convert_colorspace](ZImage::convert_colorspace) : Convert from one colorspace to another
    pub fn colorspace(&self) -> ZImageColorSpace {
        ZImageColorSpace::from(self.image.get_colorspace())
    }
    /// Convert from one colorspace to another
    ///
    /// # Arguments
    /// - to: The new colorspace to convert to
    /// - in_place: Whether to perform the conversion in place or to create a copy and convert that
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (to, in_place = false))]
    pub fn convert_colorspace(
        &mut self, to: ZImageColorSpace, in_place: bool
    ) -> PyResult<Option<ZImage>> {
        let color = to.to_colorspace();
        exec_filter(self, ColorspaceConv::new(color), in_place)
    }
    /// Return the image depth
    ///
    /// # Returns
    /// - The image depth
    ///
    /// This also gives you the internal representation of an image
    ///  - Eight: u8 (1 byte per pixel)
    ///  - Sixteen: u16 (2 bytes per pixel)
    ///  - F32: Float f32 (4 bytes per pixel, float type)
    ///  
    pub fn depth(&self) -> ZImageDepth {
        ZImageDepth::from(self.image.get_depth())
    }
    /// Save an image to a format
    ///
    /// Not all image formats have encoders enabled
    /// so check by calling `PyImageFormat.has_encoder()` which returns a boolean
    /// indicating if the image format has an encoder
    ///
    /// # Arguments
    ///  - file: Filename to save the file to
    ///  - format:  The format to save the file in
    ///
    /// # Returns
    ///  - Nothing on success, or Exception  on error
    pub fn save(&self, file: String, format: ZImageFormats) -> PyResult<()> {
        if let Err(e) = self.image.save_to(file, format.to_imageformat()) {
            return Err(PyErr::new::<PyException, _>(format!(
                "Error encoding: {:?}",
                e
            )));
        }
        Ok(())
    }

    /// Crop an image
    ///
    /// # Arguments
    /// - width: Out width, how wide the new image should be
    /// - height: Out height, how tall the new image should be
    /// - x : How many pixels horizontally from the origin should the cropping start from
    /// - y : How many pixels vertically from the origin should the cropping start from.
    ///
    ///  - in_place: Whether to carry out the crop in place or create a clone for which to crop
    ///
    /// Origin is defined from the top left of the image.
    ///
    #[pyo3(signature = (width, height, x, y, in_place = false))]
    pub fn crop(
        &mut self, width: usize, height: usize, x: usize, y: usize, in_place: bool
    ) -> PyResult<Option<ZImage>> {
        exec_filter(self, Crop::new(width, height, x, y), in_place)
    }
    /// Transpose the image.
    ///
    /// This rewrites pixels into `dst(i,j)=src(j,i)`
    ///
    /// # Arguments
    /// - inplace: Whether to transpose the image in place or generate a clone
    /// and transpose the new clone
    #[pyo3(signature = (in_place = false))]
    pub fn transpose(&mut self, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Transpose, in_place)
    }

    /// Convert from one depth to another
    ///
    /// The following are the depth conversion details
    ///  
    /// - INT->Float : Convert to float and divide by max value for the previous integer type(255 for u8,65535 for u16).
    /// - Float->Int : Multiply by max value of the new depth (255->Eight,65535->16)
    /// - smallInt->Int :  Multiply by (MAX_LARGE_INT/MAX_SMALL_INT)
    /// - LargeInt->SmallInt: Divide by (MAX_LARGE_INT/MAX_SMALL_INT)  
    ///
    /// # Arguments
    /// - to: The new depth to convert to
    /// - in_place: Whether to perform the conversion in place or to create a copy and convert that
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (to, in_place = false))]
    pub fn convert_depth(&mut self, to: ZImageDepth, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Depth::new(to.to_depth()), in_place)
    }
    /// Applies a fixed-level threshold to each array element.
    ///
    /// Thresholding works best for grayscale images, passing a colored image
    /// does not implicitly convert it to grayscale, you need to do that explicitly
    ///
    /// # Arguments
    ///  - value: Non-zero value assigned to the pixels for which the condition is satisfied
    ///  - method: The thresholding method used, defaults to binary which generates a black
    /// and white image from a grayscale image
    ///  - in_place: Whether to perform the operation in-place or to clone and return a copy
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (value, method = ZImageThresholdType::Binary, in_place = false))]
    pub fn threshold(
        &mut self, value: f32, method: ZImageThresholdType, in_place: bool
    ) -> PyResult<Option<ZImage>> {
        exec_filter(self, Threshold::new(value, method.to_threshold()), in_place)
    }
    /// Invert (negate) an image
    ///
    /// # Arguments
    ///  - in_place: Whether to perform the operation in-place or to clone and return a copy
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (in_place = false))]
    pub fn invert(&mut self, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Invert, in_place)
    }

    /// Blur the image using a box blur operation
    ///
    /// # Arguments
    ///  - in_place: Whether to perform the operation in-place or to clone and return a copy
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (radius, in_place = false))]
    pub fn box_blur(&mut self, radius: usize, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, BoxBlur::new(radius), in_place)
    }

    /// Adjust exposure of image filter
    ///
    ///#  Arguments
    /// - exposure: Set the exposure correction, allowed range is from -3.0 to 3.0. Default should be zero
    /// - black: Set black level correction: Allowed range from -1.0 to 1.0. Default is zero.
    ///
    /// For 8 bit and 16 bit images, values are clamped to their limits,
    /// for floating point, no clamping occurs
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (exposure, black_point = 0.0, in_place = false))]
    pub fn exposure(
        &mut self, exposure: f32, black_point: f32, in_place: bool
    ) -> PyResult<Option<ZImage>> {
        exec_filter(self, Exposure::new(exposure, black_point), in_place)
    }

    /// Creates a vertical mirror image by reflecting
    /// the pixels around the central x-axis.
    ///
    ///
    /// ```text
    ///
    ///old image     new image
    /// ┌─────────┐   ┌──────────┐
    /// │a b c d e│   │j i h g f │
    /// │f g h i j│   │e d c b a │
    /// └─────────┘   └──────────┘
    /// ```
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (in_place = false))]
    pub fn flip(&mut self, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Flip, in_place)
    }

    /// Creates a horizontal mirror image by
    /// reflecting the pixels around the central y-axis
    ///
    ///```text
    ///old image     new image
    ///┌─────────┐   ┌──────────┐
    ///│a b c d e│   │e d b c a │
    ///│f g h i j│   │j i h g f │
    ///└─────────┘   └──────────┘
    ///```
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (in_place = false))]
    pub fn flop(&mut self, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Flop, in_place)
    }
    /// Gamma adjust an image
    ///
    /// This currently only supports 8 and 16 bit depth images since it applies an optimization
    /// that works for those depths.
    ///
    /// # Arguments
    /// - gamma: Ranges typical range is from 0.8-2.3
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (gamma, in_place = false))]
    pub fn gamma(&mut self, gamma: f32, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Gamma::new(gamma), in_place)
    }

    /// Blur the image using a gaussian blur filter
    ///
    /// # Arguments
    ///   - sigma: Strength of blur
    ///  - in_place: Whether to perform the operation in-place or to clone and return a copy
    ///
    /// # Returns
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (sigma, in_place = false))]
    pub fn gaussian_blur(&mut self, sigma: f32, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, GaussianBlur::new(sigma), in_place)
    }

    /// Auto orient the image based on the exif metadata
    ///
    ///
    /// This operation is also a no-op if the image does not have
    /// exif metadata
    #[pyo3(signature = (in_place = false))]
    pub fn auto_orient(&mut self, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, AutoOrient, in_place)
    }

    /// Calculate the sobel derivative of an image
    ///
    /// This uses the standard 3x3 [Gx and Gy matrix](https://en.wikipedia.org/wiki/Sobel_operator)
    ///
    /// Gx matrix
    /// ```text
    ///   -1, 0, 1,
    ///   -2, 0, 2,
    ///   -1, 0, 1
    /// ```
    /// Gy matrix
    /// ```text
    /// -1,-2,-1,
    ///  0, 0, 0,
    ///  1, 2, 1
    /// ```
    ///
    ///  # Arguments
    /// - in-place: Whether to carry the operation in place or clone and operate on the copy
    #[pyo3(signature = (in_place = false))]
    pub fn sobel(&mut self, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Sobel, in_place)
    }
    /// Calculate the scharr derivative of an image
    ///
    /// The image is convolved with the following 3x3 matrix
    ///
    ///
    /// Gx matrix
    /// ```text
    ///   -3, 0,  3,
    ///  -10, 0, 10,
    ///   -3, 0,  3
    /// ```
    /// Gy matrix
    /// ```text
    /// -3,-10,-3,
    ///  0,  0, 0,
    ///  3, 10, 3
    /// ```
    ///
    ///  # Arguments
    /// - in-place: Whether to carry the operation in place or clone and operate on the copy
    #[pyo3(signature = (in_place = false))]
    pub fn scharr(&mut self, in_place: bool) -> PyResult<Option<ZImage>> {
        exec_filter(self, Scharr, in_place)
    }

    /// Linearly stretches the contrast in an image in place,
    /// sending lower to image minimum and upper to image maximum.
    ///
    /// Arguments:
    ///
    /// - lower: Lower minimum value for which pixels below this are clamped to the value
    /// - upper: Upper maximum value for which pixels above are clamped to the value
    ///
    ///
    /// Returns:
    ///
    ///  - If `in_place=True`: Nothing on success, on error returns error that occurred
    ///  - If `in_place=False`: An image copy on success on error, returns error that occurred
    #[pyo3(signature = (lower, upper, in_place = false))]
    pub fn stretch_contrast(
        &mut self, lower: f32, upper: f32, in_place: bool
    ) -> PyResult<Option<ZImage>> {
        let stretch_contrast = StretchContrast::new(lower, upper);

        exec_filter(self, stretch_contrast, in_place)
    }
    /// Convert the image bytes to a numpy array
    ///
    /// The array will always be a 3-D numpy array of
    /// `[width,height,colorspace_components]` dimensions/
    /// This means that e.g for a 256x256 rgb image the result will be `[256,256,3]` dimensions
    ///
    /// Colorspace is important in determining output.
    ///
    /// RGB colorspace is arranged as `R`,`G`,`B` , BGR is arranged as `B`,`G`,`R`
    ///
    ///
    /// Array type:
    ///
    /// The array type is determined by the  image depths/ image bit-type
    ///
    /// The following mappings are considered.
    ///
    /// - ZImageDepth::Eight -> dtype=uint8
    /// - ZImageDepth::Sixteen -> dtype=uint16
    /// - ZimageDepth::F32  -> dtype=float32
    ///
    ///
    /// Returns:
    ///
    ///  A numpy representation of the image if okay.
    ///
    /// An error in case something went wrong
    pub fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<&'py PyUntypedArray> {
        match self.image.get_depth().bit_type() {
            BitType::U8 => Ok(self
                .to_numpy_generic::<u8>(py, ZImageDepth::U8)?
                .as_untyped()),
            BitType::U16 => Ok(self
                .to_numpy_generic::<u16>(py, ZImageDepth::U16)?
                .as_untyped()),
            BitType::F32 => Ok(self
                .to_numpy_generic::<f32>(py, ZImageDepth::F32)?
                .as_untyped()),
            d => Err(PyErr::new::<PyException, _>(format!(
                "Error converting to depth {:?}",
                d
            )))
        }
    }
}

#[pyfunction]
pub fn decode_image(bytes: &[u8]) -> PyResult<ZImage> {
    let im_result = Image::read(bytes, DecoderOptions::new_fast());
    return match im_result {
        Ok(result) => Ok(ZImage::new(result)),
        Err(err) => Err(PyErr::new::<PyException, _>(format!(
            "Error decoding: {:?}",
            err
        )))
    };
}

impl From<ZImageErrors> for pyo3::PyErr {
    fn from(value: ZImageErrors) -> Self {
        PyErr::new::<PyException, _>(format!("{:?}", value.error))
    }
}

/// Decode a file path containing an image
#[pyfunction]
pub fn decode_file(file: String) -> PyResult<ZImage> {
    return match read(file) {
        Ok(bytes) => Ok(ZImage::new(
            Image::read(bytes, DecoderOptions::new_fast()).map_err(|x| ZImageErrors::from(x))?
        )),
        Err(e) => Err(PyErr::new::<PyException, _>(format!("{}", e)))
    };
}
