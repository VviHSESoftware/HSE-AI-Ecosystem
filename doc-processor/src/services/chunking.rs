use crate::schemas::{PlainChunk, ChunkTextRequest, PartitionedChunk, ChunkPartitionedTextRequest};
use text_splitter::{TextSplitter, ChunkConfig};

pub struct ChunkingService  {

}

impl ChunkingService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn chunk_text(&self, req: ChunkTextRequest) -> Vec<PlainChunk> {
        let config = ChunkConfig::new(req.max_chunk_size)
            .with_overlap(req.overlap.round() as usize)
            .unwrap_or(ChunkConfig::new(req.max_chunk_size));

        let splitter = TextSplitter::new(config);

        splitter.chunks(&req.text)
            .map(|chunk| PlainChunk { text: chunk.to_string() })
            .collect()
    }

    pub fn chunk_partitioned_text(&self, req: ChunkPartitionedTextRequest) -> Vec<PartitionedChunk> {
        if req.parts.is_empty() { return vec![]; }

        let mut concat_text = String::new();
        let mut byte_offsets = Vec::new();
        let mut start_values = Vec::new();
        let mut end_values = Vec::new();

        for part in &req.parts {
            byte_offsets.push(concat_text.len());
            start_values.push(part.start);
            end_values.push(part.end);
            concat_text.push_str(&part.text);
            concat_text.push(' ');
        }

        let config = ChunkConfig::new(req.max_chunk_size)
            .with_overlap(req.overlap.round() as usize)
            .unwrap_or(ChunkConfig::new(req.max_chunk_size));

        let splitter = TextSplitter::new(config);

        let mut chunks = Vec::new();

        for (byte_offset, chunk_text) in splitter.chunk_indices(&concat_text) {
            let chunk_end_byte = byte_offset + chunk_text.len();

            let start_idx = byte_offsets.partition_point(|&offset| offset <= byte_offset).saturating_sub(1);
            let chunk_start_val = start_values[start_idx];

            let end_idx_search = byte_offsets.partition_point(|&offset| offset <= chunk_end_byte).saturating_sub(1);
            let chunk_end_val = end_values[end_idx_search];

            chunks.push(PartitionedChunk {
                start: chunk_start_val,
                end: chunk_end_val,
                text: chunk_text.to_string(),
            });
        }
        chunks
    }
}