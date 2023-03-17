use lsp_types::Location;

pub trait GetPosition {
    fn get_position(&self) -> (url::Url, u32 /* line */, u32 /* col */);
    fn in_range(x: &impl GetPosition, range: &Location) -> bool {
        let (filepath, line, col) = x.get_position();
        if filepath != range.uri {
            return false;
        }
        if line < range.range.start.line {
            return false;
        }
        if line == range.range.start.line && col < range.range.start.character {
            return false;
        }
        if line > range.range.end.line {
            return false;
        }
        if line == range.range.end.line && col > range.range.end.character {
            return false;
        }
        true
    }
}
