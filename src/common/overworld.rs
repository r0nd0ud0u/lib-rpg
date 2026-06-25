use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Position { x, y }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileKind {
    Floor,
    Wall,
    Grass,
    Water,
    Door { target_map: String, spawn: Position },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub kind: TileKind,
    pub sprite_id: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_position_new() {
        let p = Position::new(3, 7);
        assert_eq!(p.x, 3);
        assert_eq!(p.y, 7);
    }

    #[test]
    fn unit_position_default() {
        let p = Position::default();
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
    }

    #[test]
    fn unit_tile_kind_serde_floor() {
        let kind = TileKind::Floor;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"floor\"", "got: {json}");
        let back: TileKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TileKind::Floor);
    }

    #[test]
    fn unit_tile_kind_serde_door() {
        let kind = TileKind::Door {
            target_map: "route_1".to_string(),
            spawn: Position::new(1, 2),
        };
        let json = serde_json::to_string(&kind).unwrap();
        let back: TileKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, kind);
    }

    #[test]
    fn unit_direction_variants() {
        for dir in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            let json = serde_json::to_string(&dir).unwrap();
            let back: Direction = serde_json::from_str(&json).unwrap();
            assert_eq!(back, dir);
        }
    }
}
