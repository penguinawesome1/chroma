use glam::IVec3;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BoundsError {
    #[error("Position {0:?} is out of bounds for the section.")] OutOfBounds(IVec3),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct Section<const W: usize, const H: usize, const D: usize> {
    data: Vec<u64>,
    palette: Vec<u64>,
    bits_per_item: u8,
}

impl<const W: usize, const H: usize, const D: usize> Section<W, H, D> {
    const VOLUME: usize = W * H * D;
    const BITS_PER_WORD: usize = 64;

    /// Creates a new section given dimensions and initial bits per item.
    ///
    /// The more bits per item the more memory but less likely to repack.
    ///
    /// # Examples
    ///
    /// ```
    /// use glam::IVec3;
    /// use chroma::Section;
    ///
    /// let mut section: Section<16, 16, 16> = Section::new(2);
    /// assert!(section.is_empty());
    ///
    /// let pos: IVec3 = IVec3::new(0, 0, 0);
    /// section.set_item(pos, 2);
    /// assert!(!section.is_empty());
    /// ```
    pub fn new(bits_per_item: u8) -> Self {
        let palette_len: usize = 1 << bits_per_item;
        let total_bits_needed: usize = (bits_per_item as usize) * Self::VOLUME;
        let data_len: usize = (total_bits_needed + Self::BITS_PER_WORD - 1) / Self::BITS_PER_WORD;

        Self { data: vec![0; data_len], palette: vec![0; palette_len], bits_per_item }
    }

    /// Returns if there is only one item type and it has a value of zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|&word| word == 0)
    }

    /// Returns the dimensions (width, height, depth) of the section.
    #[inline]
    pub const fn dimensions(&self) -> IVec3 {
        IVec3::new(W as i32, H as i32, D as i32)
    }

    /// Returns the total number of items in the section.
    #[inline]
    pub const fn volume(&self) -> usize {
        Self::VOLUME
    }

    /// Gets an item given its three dimensional position.
    #[inline]
    pub fn item(&self, pos: IVec3) -> Result<u64, BoundsError> {
        Self::check_position_in_bounds(pos)?;
        Ok(unsafe { self.item_unchecked(pos) })
    }

    /// Gets an item given its three dimensional position.
    ///
    /// # Panics
    ///
    /// Will be unchecked and may panic if position is out of bounds.
    #[inline]
    pub unsafe fn item_unchecked(&self, pos: IVec3) -> u64 {
        let item_index: usize = Self::item_index(pos);
        let palette_index: usize = self.palette_index(item_index);
        unsafe { *self.palette.get_unchecked(palette_index) }
    }

    /// Sets an item at the given three dimensional position.
    /// Returns an error if position is out of the section bounds.
    #[must_use]
    pub fn set_item(&mut self, pos: IVec3, item: u64) -> Result<(), BoundsError> {
        Self::check_position_in_bounds(pos)?;
        unsafe {
            self.set_item_unchecked(pos, item);
        }
        Ok(())
    }

    /// Sets an item at the given three dimensional position.
    ///
    /// # Panics
    ///
    /// Will be unchecked and may panic if position is out of bounds.
    pub unsafe fn set_item_unchecked(&mut self, pos: IVec3, item: u64) {
        let palette_index = self.palette
            .iter()
            .position(|&id| id == item)
            .unwrap_or_else(|| {
                let new_index: usize = self.palette.len();
                self.palette.push(item);

                if 1 << self.bits_per_item <= new_index {
                    self.repack(self.bits_per_item + 1);
                }

                new_index
            });

        let item_index: usize = Self::item_index(pos);

        unsafe {
            self.set_item_ex(item_index, palette_index);
        }
    }

    unsafe fn set_item_ex(&mut self, item_index: usize, palette_index: usize) {
        debug_assert!(palette_index < 1usize << self.bits_per_item, "repack needed first");

        let (word_index, bit_in_word) = Self::split_index(item_index, self.bits_per_item);
        let bits_in_first_word: usize = Self::BITS_PER_WORD - bit_in_word;

        unsafe {
            if (self.bits_per_item as usize) <= bits_in_first_word {
                let item_mask: u64 = (1u64 << self.bits_per_item).wrapping_sub(1);
                *self.data.get_unchecked_mut(word_index) &= !(item_mask << bit_in_word);
                *self.data.get_unchecked_mut(word_index) |=
                    ((palette_index as u64) & item_mask) << bit_in_word;
            } else {
                let bits_in_second_word: usize = (self.bits_per_item as usize) - bits_in_first_word;
                let mask_for_first_word: u64 = (1u64 << bits_in_first_word).wrapping_sub(1);
                *self.data.get_unchecked_mut(word_index) &= !(mask_for_first_word << bit_in_word);
                *self.data.get_unchecked_mut(word_index) |=
                    ((palette_index as u64) & mask_for_first_word) << bit_in_word;

                debug_assert!(
                    word_index + 1 < self.data.len(),
                    "should not write beyond data bounds"
                );

                let mask_for_second_word: u64 = (1u64 << bits_in_second_word).wrapping_sub(1);
                *self.data.get_unchecked_mut(word_index + 1) &= !mask_for_second_word;
                *self.data.get_unchecked_mut(word_index + 1) |=
                    ((palette_index as u64) >> bits_in_first_word) & mask_for_second_word;
            }
        }
    }

    #[inline]
    const fn split_index(item_index: usize, bits_per_item: u8) -> (usize, usize) {
        let bit_offset: usize = item_index * (bits_per_item as usize);
        let word_index: usize = bit_offset / Self::BITS_PER_WORD;
        let bit_in_word: usize = bit_offset % Self::BITS_PER_WORD;
        (word_index, bit_in_word)
    }

    #[inline]
    const fn item_index(pos: IVec3) -> usize {
        (pos.x as usize) * (H * D) + (pos.y as usize) * D + (pos.z as usize)
    }

    #[inline]
    fn palette_index(&self, item_index: usize) -> usize {
        let (word_index, bit_in_word) = Self::split_index(item_index, self.bits_per_item);

        let mut item: u64 = self.data[word_index];

        if bit_in_word + (self.bits_per_item as usize) > Self::BITS_PER_WORD {
            item >>= bit_in_word;
            let remaining_bits_n: usize =
                bit_in_word + (self.bits_per_item as usize) - Self::BITS_PER_WORD;
            let next_word: u64 = self.data[word_index + 1];
            item |= next_word << ((self.bits_per_item as usize) - remaining_bits_n);
        } else {
            item >>= bit_in_word;
        }

        let mask: u64 = (1 << self.bits_per_item) - 1;
        (item & mask) as usize
    }

    // adjusts the data to account for a new amount of bits per item
    fn repack(&mut self, new_bits_per_item: u8) {
        debug_assert!(self.bits_per_item <= new_bits_per_item, "repack must increase bits");

        let all_palette_indices: Vec<usize> = (0..Self::VOLUME)
            .map(|item_index| self.palette_index(item_index))
            .collect();

        self.bits_per_item = new_bits_per_item;
        let new_total_bits_needed: usize = (self.bits_per_item as usize) * Self::VOLUME;
        let new_data_len: usize =
            (new_total_bits_needed + Self::BITS_PER_WORD - 1) / Self::BITS_PER_WORD;
        self.data = vec![0; new_data_len];

        for item_index in 0..Self::VOLUME {
            unsafe {
                let palette_index: usize = *all_palette_indices.get_unchecked(item_index);
                self.set_item_ex(item_index, palette_index);
            }
        }
    }

    #[must_use]
    const fn check_position_in_bounds(pos: IVec3) -> Result<(), BoundsError> {
        if
            pos.x < 0 ||
            pos.x >= (W as i32) ||
            pos.y < 0 ||
            pos.y >= (H as i32) ||
            pos.z < 0 ||
            pos.z >= (D as i32)
        {
            Err(BoundsError::OutOfBounds(pos))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_empty() {
        let section: Section<16, 16, 16> = Section::new(2);
        assert!(section.is_empty());
    }

    #[test]
    fn test_set_and_get_item() {
        let mut section: Section<16, 16, 16> = Section::new(4);
        let pos_1: IVec3 = IVec3::new(15, 1, 1);
        let pos_2: IVec3 = IVec3::new(15, 1, 2);

        unsafe {
            section.set_item_unchecked(pos_1, 3);
            section.set_item_unchecked(pos_1, 2);
            section.set_item_unchecked(pos_2, 1);

            assert_eq!(section.item_unchecked(pos_1), 2);
            assert_eq!(section.item_unchecked(pos_2), 1);
        }
    }

    #[test]
    fn test_repack() {
        let mut section: Section<16, 16, 16> = Section::new(1);
        let pos: IVec3 = IVec3::new(3, 5, 3);

        unsafe {
            section.set_item_unchecked(pos, 30);
            assert_eq!(section.item_unchecked(pos), 30);
        }
    }
}
