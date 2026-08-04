/// SSE parser helper function.
pub fn parse_sse_line(line: &str) -> Option<&str> {
    if line == "data: [DONE]" {
        return Some("[DONE]");
    }
    line.strip_prefix("data: ").map(str::trim)
}
