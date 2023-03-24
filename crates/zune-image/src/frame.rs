//! A single image frame

#![allow(dead_code)]

use std::any::TypeId;

use zune_core::colorspace::ColorSpace;

use crate::channel::Channel;
use crate::errors::ImageErrors;
use crate::traits::ZuneInts;

/// A single image frame
///
/// This represents a simple image frame which contains a group
/// of channels whose metadata is contained by the
/// parent image struct.
#[derive(Clone)]
pub struct Frame
{
    pub(crate) channels: Vec<Channel>,
    pub(crate) duration: u64
}

impl Frame
{
    /// Create a new frame with default duration of 0
    ///
    /// # Arguments
    ///
    /// * `channels`:  Image channels for this frame
    ///
    /// returns: Frame
    ///
    /// # Examples
    ///
    /// ```
    /// use zune_image::channel::Channel;
    /// use zune_image::frame::Frame;
    /// // create a group of channels, this should
    /// // represent de-interleaved/single channel image contents
    /// let channel = vec![Channel::new();3];
    /// // create a new frame  
    /// let frame = Frame::new(channel);
    ///
    /// ```
    pub fn new(channels: Vec<Channel>) -> Frame
    {
        Frame {
            channels,
            duration: 0
        }
    }

    /// Create a new frame with specified duration
    ///
    /// # Arguments
    ///
    /// * `channels`:  Channels for this frame
    /// * `duration`:  How long we wait for transition of this frame to another frame
    ///
    /// returns: Frame, with the duration
    ///
    /// # Examples
    ///
    /// ```
    /// use zune_image::channel::Channel;
    /// use zune_image::frame::Frame;
    /// let channels = vec![Channel::new();3];
    /// // create a new frame
    /// let frame = Frame::new_with_duration(channels,60);
    ///
    /// ```
    pub fn new_with_duration(channels: Vec<Channel>, duration: u64) -> Frame
    {
        Frame { channels, duration }
    }

    /// Return a reference to the underlying channels
    pub fn get_channels_ref(&self, colorspace: ColorSpace, ignore_alpha: bool) -> &[Channel]
    {
        // check if alpha channel is present in colorspace
        if ignore_alpha && colorspace.has_alpha()
        {
            // do not take the last one,
            // we assume the last one contains the alpha channel
            // in it.
            // TODO: Is this a bad assumption
            &self.channels[0..colorspace.num_components() - 1]
        }
        else
        {
            &self.channels[0..colorspace.num_components()]
        }
    }
    /// Return a reference to the underlying channels
    pub fn get_channels_mut(&mut self, colorspace: ColorSpace, ignore_alpha: bool)
        -> &mut [Channel]
    {
        // check if alpha channel is present in colorspace
        if ignore_alpha && colorspace.has_alpha()
        {
            // do not take the last one,
            // we assume the last one contains the alpha channel
            // in it.
            // TODO: Is this a bad assumption
            &mut self.channels[0..colorspace.num_components() - 1]
        }
        else
        {
            &mut self.channels[0..colorspace.num_components()]
        }
    }
    pub fn add(&mut self, channel: Channel)
    {
        self.channels.push(channel)
    }

    ///  Flatten all
    ///
    /// # Arguments
    ///
    /// * `colorspace`:
    /// * `out_pixel`:
    ///
    /// returns: Result<(), ImageErrors>
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub fn flatten_rgba(
        &mut self, colorspace: ColorSpace, out_pixel: &mut [u8]
    ) -> Result<(), ImageErrors>
    {
        // confirm all channels are in u8
        for channel in &self.channels
        {
            if channel.get_type_id() != TypeId::of::<u8>()
            {
                // wrong type id, that's an error
                return Err(ImageErrors::WrongTypeId(
                    channel.get_type_id(),
                    TypeId::of::<u8>()
                ));
            }
        }

        match colorspace.num_components()
        {
            1 =>
            {
                let luma_channel = self.channels[0].reinterpret_as::<u8>().unwrap();

                for (out, luma) in out_pixel.chunks_exact_mut(4).zip(luma_channel)
                {
                    out[0] = *luma;
                    out[1] = *luma;
                    out[2] = *luma;
                    out[3] = 255;
                }
            }
            2 =>
            {
                let luma_channel = self.channels[0].reinterpret_as::<u8>().unwrap();
                let alpha_channel = self.channels[1].reinterpret_as::<u8>().unwrap();

                for ((out, luma), alpha) in out_pixel
                    .chunks_exact_mut(4)
                    .zip(luma_channel)
                    .zip(alpha_channel)
                {
                    out[0] = *luma;
                    out[1] = *luma;
                    out[2] = *luma;
                    out[3] = *alpha;
                }
            }
            3 =>
            {
                let c1 = self.channels[0].reinterpret_as::<u8>().unwrap();
                let c2 = self.channels[1].reinterpret_as::<u8>().unwrap();
                let c3 = self.channels[2].reinterpret_as::<u8>().unwrap();

                for (((out, first), second), third) in
                    out_pixel.chunks_exact_mut(4).zip(c1).zip(c2).zip(c3)
                {
                    out[0] = *first;
                    out[1] = *second;
                    out[2] = *third;
                    out[3] = 255;
                }
            }
            4 =>
            {
                let c1 = self.channels[0].reinterpret_as::<u8>().unwrap();
                let c2 = self.channels[1].reinterpret_as::<u8>().unwrap();
                let c3 = self.channels[2].reinterpret_as::<u8>().unwrap();
                let c4 = self.channels[3].reinterpret_as::<u8>().unwrap();

                for ((((out, first), second), third), fourth) in out_pixel
                    .chunks_exact_mut(4)
                    .zip(c1)
                    .zip(c2)
                    .zip(c3)
                    .zip(c4)
                {
                    out[0] = *first;
                    out[1] = *second;
                    out[2] = *third;
                    out[3] = *fourth;
                }
            }
            // panics, all the way down
            _ => unreachable!()
        }
        Ok(())
    }
    pub fn flatten<T: Clone + Default + ZuneInts<T> + 'static + Copy>(
        &self, colorspace: ColorSpace
    ) -> Vec<T>
    {
        let out_pixels = match colorspace.num_components()
        {
            1 => self.channels[0].reinterpret_as::<T>().unwrap().to_vec(),

            2 =>
            {
                let luma_channel = self.channels[0].reinterpret_as::<T>().unwrap();
                let alpha_channel = self.channels[1].reinterpret_as::<T>().unwrap();

                luma_channel
                    .iter()
                    .zip(alpha_channel)
                    .flat_map(|(x1, x2)| [*x1, *x2])
                    .collect::<Vec<T>>()
            }
            3 =>
            {
                let c1 = self.channels[0].reinterpret_as::<T>().unwrap();
                let c2 = self.channels[1].reinterpret_as::<T>().unwrap();
                let c3 = self.channels[2].reinterpret_as::<T>().unwrap();

                c1.iter()
                    .zip(c2)
                    .zip(c3)
                    .flat_map(|((x1, x2), x3)| [*x1, *x2, *x3])
                    .collect::<Vec<T>>()
            }
            4 =>
            {
                let c1 = self.channels[0].reinterpret_as::<T>().unwrap();
                let c2 = self.channels[1].reinterpret_as::<T>().unwrap();
                let c3 = self.channels[2].reinterpret_as::<T>().unwrap();
                let c4 = self.channels[3].reinterpret_as::<T>().unwrap();

                c1.iter()
                    .zip(c2)
                    .zip(c3)
                    .zip(c4)
                    .flat_map(|(((x1, x2), x3), x4)| [*x1, *x2, *x3, *x4])
                    .collect::<Vec<T>>()
            }
            // panics, all the way down
            _ => unreachable!()
        };

        out_pixels
    }

    /// convert type to native endian
    pub fn u16_to_native_endian(&self, colorspace: ColorSpace) -> Vec<u8>
    {
        // confirm all channels are in u16
        for channel in &self.channels
        {
            if channel.get_type_id() != TypeId::of::<u16>()
            {
                panic!("Wrong type ID, expected u16 but got another type");
                // wrong type id, that's an error
                //return Err(ImageErrors::WrongTypeId(channel.get_type_id(), U16_TYPE_ID));
            }
        }
        let length = self.channels[0].len() * colorspace.num_components();

        let mut out_pixel = vec![0_u8; length];

        match colorspace.num_components()
        {
            // reinterpret as u16 first then native endian
            1 => self.channels[0]
                .reinterpret_as::<u16>()
                .unwrap()
                .iter()
                .zip(out_pixel.chunks_exact_mut(2))
                .for_each(|(x, y)| y.copy_from_slice(&x.to_ne_bytes())),

            2 =>
            {
                let luma_channel = self.channels[0].reinterpret_as::<u16>().unwrap();
                let alpha_channel = self.channels[1].reinterpret_as::<u16>().unwrap();

                for ((out, luma), alpha) in out_pixel
                    .chunks_exact_mut(4)
                    .zip(luma_channel)
                    .zip(alpha_channel)
                {
                    out[0..2].copy_from_slice(&luma.to_ne_bytes());
                    out[2..4].copy_from_slice(&alpha.to_ne_bytes());
                }
            }
            3 =>
            {
                let c1 = self.channels[0].reinterpret_as::<u16>().unwrap();
                let c2 = self.channels[1].reinterpret_as::<u16>().unwrap();
                let c3 = self.channels[2].reinterpret_as::<u16>().unwrap();

                for (((out, first), second), third) in
                    out_pixel.chunks_exact_mut(6).zip(c1).zip(c2).zip(c3)
                {
                    out[0..2].copy_from_slice(&first.to_ne_bytes());
                    out[2..4].copy_from_slice(&second.to_ne_bytes());
                    out[4..6].copy_from_slice(&third.to_ne_bytes());
                }
            }
            4 =>
            {
                let c1 = self.channels[0].reinterpret_as::<u16>().unwrap();
                let c2 = self.channels[1].reinterpret_as::<u16>().unwrap();
                let c3 = self.channels[2].reinterpret_as::<u16>().unwrap();
                let c4 = self.channels[3].reinterpret_as::<u16>().unwrap();

                for ((((out, first), second), third), fourth) in out_pixel
                    .chunks_exact_mut(8)
                    .zip(c1)
                    .zip(c2)
                    .zip(c3)
                    .zip(c4)
                {
                    out[0..2].copy_from_slice(&first.to_ne_bytes());
                    out[2..4].copy_from_slice(&second.to_ne_bytes());
                    out[4..6].copy_from_slice(&third.to_ne_bytes());
                    out[6..8].copy_from_slice(&fourth.to_ne_bytes());
                }
            }
            // panics, all the way down
            _ => unreachable!()
        }
        out_pixel
    }

    pub fn set_channels(&mut self, channels: Vec<Channel>)
    {
        self.channels = channels;
    }
}

mod tests
{
    use zune_core::colorspace::ColorSpace;

    use crate::channel::Channel;
    use crate::frame::Frame;

    #[test]
    fn test_conversion_to_native_endian()
    {
        // test that native endian conversion works for us

        let mut channel = Channel::new::<u16>();
        channel.push(50000_u16);

        let frame = Frame::new(vec![channel]);
        let frame_data = frame.u16_to_native_endian(ColorSpace::Luma);

        assert_eq!(&frame_data, &[80, 195]);
    }

    #[test]
    fn test_flatten_grayscale_to_rgba()
    {
        let mut channel = Channel::new::<u8>();
        channel.extend::<u8>(&[10, 20, 20]);

        let mut out = vec![0; channel.len() * 4];

        let mut frame = Frame::new(vec![channel]);
        frame.flatten_rgba(ColorSpace::Luma, &mut out).unwrap();
        let reference = [10, 10, 10, 255, 20, 20, 20, 255, 20, 20, 20, 255];
        assert_eq!(&out, &reference);
    }
}