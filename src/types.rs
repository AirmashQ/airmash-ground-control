use airmash_protocol::Position;
use line_drawing::Bresenham;
use pathfinding::prelude::absdiff;

const BOUNDARY_X: f32 = 16384.0;
const BOUNDARY_Y: f32 = BOUNDARY_X / 2.0;
const MAP_MAX_X: isize = 512;
const MAP_MAX_Y: isize = MAP_MAX_X / 2;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapPosition {
    pub x: isize,
    pub y: isize,
}

impl MapPosition {
    pub fn new(x: isize, y: isize) -> MapPosition {
        MapPosition { x, y }
    }

    #[inline]
    pub fn is_occupied(self) -> bool {
        self.x < 0
            || self.x >= MAP_MAX_X
            || self.y < 0
            || self.y >= MAP_MAX_Y
            || crate::map::MAP[self.y as usize][self.x as usize] == 1
    }

    /// Detect the position of an obstacle between the two positions.
    pub fn obstacle_between(self, other: MapPosition) -> Option<MapPosition> {
        Bresenham::new(self.into(), other.into()).find_map(|(x, y)| {
            let pos = MapPosition::new(x, y);
            if pos.is_occupied() {
                Some(pos)
            } else {
                None
            }
        })
    }

    #[inline]
    pub fn distance(self, other: MapPosition) -> isize {
        absdiff(self.x, other.x) + absdiff(self.y, other.y)
    }

    pub fn adjacent_positions(self) -> impl Iterator<Item = MapPosition> {
        UnoccupiedMapPositionIter::new(self)
    }
}

impl From<Position> for MapPosition {
    fn from(pos: Position) -> MapPosition {
        let x = (((pos.x.inner() + BOUNDARY_X) / 64.0).abs().max(0.0) as isize).min(MAP_MAX_X - 1);
        let y = (((pos.y.inner() + BOUNDARY_Y) / 64.0).abs().max(0.0) as isize).min(MAP_MAX_Y - 1);

        MapPosition::new(x, y)
    }
}

impl From<(isize, isize)> for MapPosition {
    fn from(pos: (isize, isize)) -> MapPosition {
        MapPosition::new(pos.0, pos.1)
    }
}

impl From<MapPosition> for (isize, isize) {
    fn from(pos: MapPosition) -> (isize, isize) {
        (pos.x, pos.y)
    }
}

impl From<MapPosition> for Position {
    fn from(pos: MapPosition) -> Position {
        Position::new(
            (pos.x * 64) as f32 + 32.0 - BOUNDARY_X,
            (pos.y * 64) as f32 + 32.0 - BOUNDARY_Y,
        )
    }
}

struct UnoccupiedMapPositionIter {
    origin: MapPosition,
    x: isize,
    y: isize,
}

impl UnoccupiedMapPositionIter {
    pub fn new(origin: MapPosition) -> UnoccupiedMapPositionIter {
        UnoccupiedMapPositionIter {
            origin,
            x: origin.x - 1,
            y: origin.y - 1,
        }
    }
}

impl Iterator for UnoccupiedMapPositionIter {
    type Item = MapPosition;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let x = self.x;
            let y = self.y;

            self.x += 1;

            if !(x == self.origin.x && y == self.origin.y) {
                if x <= self.origin.x + 1 {
                    if y <= self.origin.y + 1 {
                        let pos = MapPosition::new(x, y);
                        if !pos.is_occupied() {
                            return Some(pos);
                        }
                    } else {
                        return None;
                    }
                } else {
                    self.x = self.origin.x - 1;
                    self.y += 1;
                }
            }
        }
    }
}
