//! A basic implementation of a piece table.
//! It allows you to store or delete text using the public
//! You can get the text out using the to_string method.
//! This is not really for public consumption - it hasn't got a rich API
//! and it's main purpose was for me to learn the implementation of a piece buffer.
//! Performance is unknown, but should be at worst okay - I've not focussed on it,
//! however I have done basic work to make sure the numbers of pieces and buffers don't increase
//! unnecessarily.
//!
//! It could be useful for people who want to understand how a piece buffer might be implemented
//! in Rust.
use std::cmp::{max, min};

/// A section of the buffer representing some text. Equivalent to a slice of a string.
#[derive(Copy, Clone, Debug)]
struct Piece {
    buffer_index: usize,
    start: usize,
    end: usize,
}

impl Piece {
    /// Length of the piece in bytes.
    fn len(&self) -> usize {
        self.end - self.start
    }

    /// Creates a new sub-piece running from start to start+offset.
    /// The offset is specified in bytes from the beginning of
    /// the piece.
    fn before(&self, offset: usize) -> Self {
        Piece {
            buffer_index: self.buffer_index,
            start: self.start,
            end: offset + self.start,
        }
    }

    /// Creates a new sub-piece running from start+offset to end.
    /// The offset is specified in bytes from the beginning of
    /// the piece.
    fn after(&self, offset: usize) -> Self {
        Piece {
            buffer_index: self.buffer_index,
            start: self.start + offset,
            end: self.end,
        }
    }

    /// Merges a piece into the current piece if it is mergeable.
    /// Returns true it is merges the piece, false if it cannot be merged.
    fn merge(&mut self, piece: Piece) -> bool {
        if piece.buffer_index == self.buffer_index && piece.start == self.end {
            self.end = piece.end;
            true
        } else {
            false
        }
    }
}

type Buffer = String;
pub struct PieceTable {
    buffers: Vec<Buffer>,
    pieces: Vec<Piece>,
}

/// Represents the point in the piece table, specified as the index of a piece
/// and a byte offset from the beginning of the piece.
#[derive(Clone, Copy, Debug)]
struct Location {
    piece_index: usize,
    offset: usize,
}

impl Location {
    pub fn new(index: usize, offset: usize) -> Self {
        Location {
            piece_index: index,
            offset,
        }
    }
}

impl PieceTable {
    /// Creates a new empty piece table
    pub fn new() -> Self {
        PieceTable {
            buffers: Vec::new(),
            pieces: Vec::new(),
        }
    }

    /// Creates a new piece table initialized with the specified string
    pub fn from_string(s: String) -> Self {
        PieceTable {
            pieces: vec![Piece {
                buffer_index: 0,
                start: 0,
                end: s.len(),
            }],
            buffers: vec![s],
        }
    }

    /// Adds a new buffer to the piece table with at least the same capacity
    /// as all the other buffers put together.
    fn add_buffer(&mut self, min_capacity: usize) {
        let buffer = String::with_capacity(max(
            min_capacity,
            self.buffers
                .iter()
                .fold(0, |sum, buffer| sum + buffer.len()),
        ));
        self.buffers.push(buffer);
    }

    /// Find the location of a piece in the piece table.
    fn locate(&self, position: usize) -> Location {
        let mut offset = 0;
        for (index, piece) in self.pieces.iter().copied().enumerate() {
            offset += piece.len();
            if position < offset {
                return Location::new(index, piece.len() - (offset - position));
            }
            if position == offset {
                return Location::new(index + 1, 0);
            }
        }
        return Location::new(self.pieces.len(), 0);
    }

    /// Split a piece in two at the specified point if necessary.
    /// It can also delete characters between the two pieces.
    /// loc specifies the location to delete at, and gap specifies the number
    /// number of bytes between the end of the first piece, and the start of the second.
    /// Note that it won't create unnecessary pieces if you are splitting at the beginning or
    /// the end of a piece.
    ///
    /// The return value is the insertion index required to insert a new piece in the gap.
    fn split(&mut self, loc: Location, gap: usize) -> usize {
        if let Some(piece) = self.pieces.get(loc.piece_index).copied() {
            if loc.offset == 0 {
                let after = piece.after(gap);
                if after.len() > 0 {
                    self.pieces[loc.piece_index] = after;
                } else {
                    self.pieces.remove(loc.piece_index);
                }

                return loc.piece_index;
            }
            if loc.offset < piece.len() {
                self.pieces[loc.piece_index] = piece.before(loc.offset);
                if gap + loc.offset < piece.len() {
                    self.pieces
                        .insert(loc.piece_index + 1, piece.after(loc.offset + gap));
                }
            }
            return loc.piece_index + 1;
        }
        loc.piece_index
    }

    // Retrieves a buffer with at least the specified capacity.
    fn buffer_with_capacity(&mut self, capacity: usize) -> (usize, &mut Buffer) {
        if self
            .buffers
            .last_mut()
            .filter(|buffer| buffer.capacity() - buffer.len() > capacity)
            .is_none()
        {
            self.add_buffer(capacity);
        }
        (self.buffers.len() - 1, self.buffers.last_mut().unwrap())
    }

    pub fn insert(&mut self, position: usize, s: &str) {
        let (buffer_index, buffer) = self.buffer_with_capacity(s.len());
        let start = buffer.len();
        let end = start + s.len();
        *buffer += s;

        let piece = Piece {
            buffer_index,
            start,
            end,
        };

        let loc = self.locate(position);
        let index = self.split(loc, 0);

        if index == 0 || !self.pieces[index - 1].merge(piece) {
            self.pieces.insert(index, piece);
        }
    }

    pub fn delete(&mut self, position: usize, mut len: usize) {
        let mut pos = self.locate(position);

        if pos.offset > 0 {
            let gap = min(len, self.pieces[pos.piece_index].len() - pos.offset);
            self.split(pos, len);
            len -= gap;
            pos.piece_index += 1;
            pos.offset = 0;
        }

        while len > 0 && self.pieces.len() > pos.piece_index {
            let gap = min(len, self.pieces[pos.piece_index].len());
            self.split(pos, gap);
            len -= gap;
        }
    }

    fn piece_text(&self, piece: Piece) -> &str {
        &self.buffers[piece.buffer_index][piece.start..piece.end]
    }

    pub fn to_string(&self) -> String {
        self.pieces.iter().fold(String::new(), |mut s, piece| {
            s += self.piece_text(*piece);
            s
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_should_create_a_piece_table_with_no_buffers_or_pieces() {
        let piece_table = PieceTable::new();

        assert_eq!(piece_table.pieces.len(), 0);
        assert_eq!(piece_table.buffers.len(), 0);
    }

    #[test]
    fn it_should_append_a_string_to_an_empty_piece_buffer() {
        let mut piece_table = PieceTable::new();

        piece_table.insert(0, "Hello, World");

        assert_eq!(&piece_table.to_string(), "Hello, World");
    }

    #[test]
    fn inserting_at_beginning_should_prepend_text() {
        let mut piece_table = PieceTable::new();

        piece_table.insert(0, "World");
        piece_table.insert(0, "Hello, ");

        assert_eq!(&piece_table.to_string(), "Hello, World");
    }

    #[test]
    fn inserting_at_end_should_append_text() {
        let mut piece_table = PieceTable::new();

        piece_table.insert(0, "Hello, ");
        piece_table.insert(7, "World");

        assert_eq!(&piece_table.to_string(), "Hello, World");
    }

    #[test]
    fn inserting_in_middle_should_split_original_text() {
        let mut piece_table = PieceTable::new();

        piece_table.insert(0, "Goodbye World");
        piece_table.insert(7, " cruel");

        assert_eq!(&piece_table.to_string(), "Goodbye cruel World");
    }

    #[test]
    fn delete_from_middle_removes_text() {
        let mut piece_table = PieceTable::from_string("Hello, World".to_owned());

        piece_table.delete(5, 1);

        assert_eq!(&piece_table.to_string(), "Hello World");
    }

    #[test]
    fn delete_from_start_removes_text() {
        let mut piece_table = PieceTable::from_string("Hello, World".to_owned());

        piece_table.delete(0, 7);

        assert_eq!(&piece_table.to_string(), "World");
    }

    #[test]
    fn delete_from_end_removes_text_without_adding_new_pieces() {
        let mut piece_table = PieceTable::from_string("Hello, World".to_owned());

        piece_table.delete(5, 7);

        assert_eq!(&piece_table.to_string(), "Hello");
        assert_eq!(piece_table.pieces.len(), 1);
    }

    #[test]
    fn delete_whole_piece_removes_piece() {
        let mut piece_table = PieceTable::from_string("Hello, World".to_owned());

        piece_table.delete(0, 12);

        assert_eq!(&piece_table.to_string(), "");
        assert_eq!(piece_table.pieces.len(), 0);
    }

    #[test]
    fn deleting_multiple_pieces_removes_all_pieces() {
        let mut piece_table = PieceTable::from_string("Hello World".to_owned());

        piece_table.insert(5, ",");
        assert_eq!(piece_table.pieces.len(), 3); //Quick sanity check - if we've not got 3 pieces then the test isn't valid!

        piece_table.delete(2, 10);

        assert_eq!(&piece_table.to_string(), "He");
        assert_eq!(piece_table.pieces.len(), 1);
    }

    #[test]
    fn inserting_past_end_inserts_at_end() {
        let mut piece_table = PieceTable::from_string("Hello, World".to_owned());
        piece_table.insert(500, "Boom");
        assert_eq!(&piece_table.to_string(), "Hello, WorldBoom");
    }

    #[test]
    fn deleting_when_start_is_past_end_of_buffer_does_nothing() {
        let mut piece_table = PieceTable::from_string("Hello, World".to_owned());
        piece_table.delete(500, 1);
        assert_eq!(&piece_table.to_string(), "Hello, World");
    }

    #[test]
    fn deleting_when_it_would_delete_past_the_end_deletes_to_end() {
        let mut piece_table = PieceTable::from_string("Hello, World".to_owned());
        piece_table.delete(5, 500);
        assert_eq!(&piece_table.to_string(), "Hello");
    }
}
