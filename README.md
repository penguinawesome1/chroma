# Chroma

An efficient way to store data with bitpacking.

## Features

- **Dynamic Bit Packing:** Efficiently stores data by adjusting the number of bits per item at runtime.
- **Palette System:** Reduces memory footprint by mapping unique data values to smaller indices.
- **Compile-Time Dimensions:** Define your 3D section's width, height, and depth at compile time for performance and type safety.
- **Bounds Checking:** Safe methods for setting and retrieving items with error handling for out-of-bounds access.

## Functionality

```rust
use glam::IVec3;
use chroma::Section;

// Create a new 16x16x16 section with an initial capacity for 2 bits per item
let mut section: Section<16, 16, 16> = Section::new(2);

// A newly created section is empty
assert!(section.is_empty());

// Define a 3D position
let pos: IVec3 = IVec3::new(0, 0, 0);

// Set an item at the position.
// If the item count exceeds the current bits_per_item capacity,
// the section will automatically repack to accommodate more unique items.
section.set_item(pos, 2).expect("Position was out of bounds!");

// The section is no longer empty after setting an item
assert!(!section.is_empty());

// You can also retrieve the item
let item_value = section.item(pos).expect("Position was out of bounds!");
assert_eq!(item_value, 2);
```

## Getting Started

```toml
[dependencies]
chroma = "0.1.0"
glam = "0.28"
thiserror = "1.0"
```

## License

This project is licensed under the [MIT License](LICENSE) - see the `LICENSE` file for details.
