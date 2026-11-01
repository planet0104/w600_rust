//! 乐谱，与 `buzzer` 示例相同：C 调整体升高一个八度（无源蜂鸣器在 1kHz 附近最响）。

// 各音的频率（Hz）。C5 即简谱 1=C 时升高一个八度的中音 1。
pub const G4: u32 = 392;
pub const A4: u32 = 440;
pub const B4: u32 = 494;
pub const C5: u32 = 523;
pub const D5: u32 = 587;
pub const E5: u32 = 659;
pub const F5: u32 = 698;
pub const G5: u32 = 784;
pub const A5: u32 = 880;
pub const B5: u32 = 988;
pub const C6: u32 = 1047;
pub const D6: u32 = 1175;
pub const E6: u32 = 1319;
pub const F6: u32 = 1397;
pub const G6: u32 = 1568;
pub const REST: u32 = 0;

/// 一首曲子。`notes` 为（频率，时值），时值以十六分音符为单位：
/// 1 = 十六分音符，2 = 八分音符，3 = 附点八分音符，4 = 四分音符，8 = 二分音符，12 = 附点二分音符。
pub struct Song {
    pub name: &'static str,
    /// 每分钟的拍数（四分音符）。
    pub bpm: u32,
    pub notes: &'static [(u32, u32)],
}

/// 每行一小节，开头单独一行的是弱起（不完整的第一小节）。
#[rustfmt::skip]
pub const SONGS: &[Song] = &[
    Song {
        name: "xin nian hao (Happy New Year)",
        bpm: 120,
        notes: &[
            (C5, 2), (C5, 2), (C5, 4), (G4, 4),
            (E5, 2), (E5, 2), (E5, 4), (C5, 4),
            (C5, 2), (E5, 2), (G5, 4), (G5, 4),
            (F5, 2), (E5, 2), (D5, 8),
            (D5, 2), (E5, 2), (F5, 4), (F5, 4),
            (E5, 2), (D5, 2), (E5, 4), (C5, 4),
            (C5, 2), (E5, 2), (D5, 4), (G4, 4),
            (B4, 2), (D5, 2), (C5, 8),
        ],
    },
    Song {
        name: "liang zhi lao hu (Two Tigers)",
        bpm: 120,
        notes: &[
            (C5, 4), (D5, 4), (E5, 4), (C5, 4),
            (C5, 4), (D5, 4), (E5, 4), (C5, 4),
            (E5, 4), (F5, 4), (G5, 8),
            (E5, 4), (F5, 4), (G5, 8),
            (G5, 2), (A5, 2), (G5, 2), (F5, 2), (E5, 4), (C5, 4),
            (G5, 2), (A5, 2), (G5, 2), (F5, 2), (E5, 4), (C5, 4),
            (C5, 4), (G4, 4), (C5, 8),
            (C5, 4), (G4, 4), (C5, 8),
        ],
    },
    Song {
        name: "Happy Birthday",
        bpm: 100,
        notes: &[
            (G4, 3), (G4, 1),
            (A4, 4), (G4, 4), (C5, 4),
            (B4, 8), (G4, 3), (G4, 1),
            (A4, 4), (G4, 4), (D5, 4),
            (C5, 8), (G4, 3), (G4, 1),
            (G5, 4), (E5, 4), (C5, 4),
            (B4, 4), (A4, 4), (F5, 3), (F5, 1),
            (E5, 4), (C5, 4), (D5, 4),
            (C5, 12),
        ],
    },
    Song {
        name: "It's a Small World",
        bpm: 174,
        notes: &[
            // It's a | world of laugh-ter, a | world of tears, it's a
            (E5, 2), (F5, 2),
            (G5, 4), (E6, 4), (C6, 4), (D6, 2), (C6, 2),
            (C6, 4), (B5, 4), (B5, 4), (D5, 2), (E5, 2),
            // world of hopes and a | world of fears, there's so
            (F5, 4), (D6, 4), (B5, 4), (C6, 2), (B5, 2),
            (A5, 4), (G5, 4), (G5, 4), (E5, 2), (F5, 2),
            // much that we share that it's | time we're a-ware it's a
            (G5, 4), (C6, 2), (D6, 2), (E6, 4), (D6, 2), (C6, 2),
            (A5, 4), (D6, 2), (E6, 2), (F6, 4), (E6, 2), (D6, 2),
            // small world af-ter | all.
            (G5, 4), (F6, 4), (E6, 4), (D6, 4),
            (C6, 16),
            // 副歌：It's a small world | af-ter all（"It's" 和 "af" 为附点四分音符）
            (C6, 6), (C6, 2), (E6, 4), (C6, 4),
            (D6, 6), (D6, 2), (D6, 8),
            (D6, 6), (D6, 2), (F6, 4), (D6, 4),
            (E6, 6), (E6, 2), (E6, 8),
            (E6, 6), (E6, 2), (G6, 4), (E6, 4),
            (F6, 6), (F6, 2), (F6, 4), (E6, 2), (D6, 2),
            // small, small | world.
            (G5, 8), (B5, 8),
            (C6, 16),
        ],
    },
    Song {
        name: "London Bridge Is Falling Down",
        bpm: 100,
        notes: &[
            (G5, 3), (A5, 1), (G5, 2), (F5, 2),
            (E5, 2), (F5, 2), (G5, 4),
            (D5, 2), (E5, 2), (F5, 4),
            (E5, 2), (F5, 2), (G5, 4),
            (G5, 3), (A5, 1), (G5, 2), (F5, 2),
            (E5, 2), (F5, 2), (G5, 4),
            (D5, 4), (G5, 4),
            (E5, 4), (C5, 4),
        ],
    },
];
