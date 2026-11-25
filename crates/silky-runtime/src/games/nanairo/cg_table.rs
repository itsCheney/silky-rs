//! The CG table of the game: one row for each picture the CG screen can show.
//!
//! A row is the flag byte that says the picture was seen and the picture
//! with its layers in slot order (names lower case, without ".akb"). Rows
//! are in the game's order. The flags run from 3200 but are not always
//! 3200 + row: the row after 3692 holds 3694.

/// One picture of the CG table.
pub struct Row {
    /// Byte flag set when the picture has been shown.
    pub flag: u16,
    /// The base picture, then up to four layers.
    pub layers: &'static [&'static str],
}

/// Every row of the table.
pub const ROWS: [Row; 644] = [
    Row {
        flag: 3200,
        layers: &["hg090001"],
    },
    Row {
        flag: 3201,
        layers: &["hg090002"],
    },
    Row {
        flag: 3202,
        layers: &["hg090003"],
    },
    Row {
        flag: 3203,
        layers: &["hg070000"],
    },
    Row {
        flag: 3204,
        layers: &["hg080001_h01"],
    },
    Row {
        flag: 3205,
        layers: &["hg080001_h01", "hg080001_h02"],
    },
    Row {
        flag: 3206,
        layers: &["hg080002_h01"],
    },
    Row {
        flag: 3207,
        layers: &["hg080002_h01", "hg080001_h02"],
    },
    Row {
        flag: 3208,
        layers: &["hg010000"],
    },
    Row {
        flag: 3209,
        layers: &["hg080101_h01"],
    },
    Row {
        flag: 3210,
        layers: &["hg080101_h01", "hg080101_h02"],
    },
    Row {
        flag: 3211,
        layers: &["hg080101_h01", "hg080101_h03"],
    },
    Row {
        flag: 3212,
        layers: &["hg080101_h01", "hg080102_h01"],
    },
    Row {
        flag: 3213,
        layers: &["hg080101_h01", "hg080105_h01"],
    },
    Row {
        flag: 3214,
        layers: &["hg080101_h01", "hg080105_h02"],
    },
    Row {
        flag: 3215,
        layers: &["hg080101_h01", "hg080105_h03"],
    },
    Row {
        flag: 3216,
        layers: &["hg080101_h01", "hg080103_h02"],
    },
    Row {
        flag: 3217,
        layers: &["hg080101_h01", "hg080104_h02"],
    },
    Row {
        flag: 3218,
        layers: &["hg080101_h01", "hg080104_h01"],
    },
    Row {
        flag: 3219,
        layers: &["hg080101_h01", "hg080104_h04"],
    },
    Row {
        flag: 3220,
        layers: &["hg080101_h01", "hg080104_h03"],
    },
    Row {
        flag: 3221,
        layers: &["hg080201_h01"],
    },
    Row {
        flag: 3222,
        layers: &["hg080201_h01", "hg080201_h02"],
    },
    Row {
        flag: 3223,
        layers: &["hg080202_h01"],
    },
    Row {
        flag: 3224,
        layers: &["hg080202_h01", "hg080201_h03"],
    },
    Row {
        flag: 3225,
        layers: &["hg080202_h01", "hg080201_h02"],
    },
    Row {
        flag: 3226,
        layers: &["hg080202_h01", "hg080201_h05"],
    },
    Row {
        flag: 3227,
        layers: &["hg080202_h01", "hg080201_h04"],
    },
    Row {
        flag: 3228,
        layers: &["hg080202_h01", "hg080201_h06"],
    },
    Row {
        flag: 3229,
        layers: &["hg080301_h01"],
    },
    Row {
        flag: 3230,
        layers: &["hg080301_h01", "hg080301_h02"],
    },
    Row {
        flag: 3231,
        layers: &["hg080301_h01", "hg080301_h03"],
    },
    Row {
        flag: 3232,
        layers: &["hg080302_h01"],
    },
    Row {
        flag: 3233,
        layers: &["hg080302_h01", "hg080301_h04"],
    },
    Row {
        flag: 3234,
        layers: &["hg040001_h01"],
    },
    Row {
        flag: 3235,
        layers: &["hg040003_h01"],
    },
    Row {
        flag: 3236,
        layers: &["hg040004_h01"],
    },
    Row {
        flag: 3237,
        layers: &["hg040002_h01"],
    },
    Row {
        flag: 3238,
        layers: &["hg010105"],
    },
    Row {
        flag: 3239,
        layers: &["hg010102"],
    },
    Row {
        flag: 3240,
        layers: &["hg010107"],
    },
    Row {
        flag: 3241,
        layers: &["hg010106"],
    },
    Row {
        flag: 3242,
        layers: &["hg010104"],
    },
    Row {
        flag: 3243,
        layers: &["hg010200_h01"],
    },
    Row {
        flag: 3244,
        layers: &["hg010200_h01", "hg010200_h02"],
    },
    Row {
        flag: 3245,
        layers: &["hg010200_h01", "hg010200_h03"],
    },
    Row {
        flag: 3246,
        layers: &["hg090101_h01"],
    },
    Row {
        flag: 3247,
        layers: &["hg090101_h01", "hg090101_h02"],
    },
    Row {
        flag: 3248,
        layers: &["hg090102_h01"],
    },
    Row {
        flag: 3249,
        layers: &["hg090103_h01"],
    },
    Row {
        flag: 3250,
        layers: &["hg090106"],
    },
    Row {
        flag: 3251,
        layers: &["hg090104_h01"],
    },
    Row {
        flag: 3252,
        layers: &["hg090104_h01", "hg090103_h02"],
    },
    Row {
        flag: 3253,
        layers: &["hg090105_h01"],
    },
    Row {
        flag: 3254,
        layers: &["hg080401_h01"],
    },
    Row {
        flag: 3255,
        layers: &["hg080401_h01", "hg080401_h02"],
    },
    Row {
        flag: 3256,
        layers: &["hg080401_h01", "hg080401_h02", "hg080401_h04"],
    },
    Row {
        flag: 3257,
        layers: &["hg080401_h01", "hg080401_h04"],
    },
    Row {
        flag: 3258,
        layers: &["hg080401_h01", "hg080401_h03"],
    },
    Row {
        flag: 3259,
        layers: &["hg080402_h01"],
    },
    Row {
        flag: 3260,
        layers: &["hg080402_h01", "hg080402_h02"],
    },
    Row {
        flag: 3261,
        layers: &["hg080403_h02"],
    },
    Row {
        flag: 3262,
        layers: &["hg080404_h01"],
    },
    Row {
        flag: 3263,
        layers: &["hg080404_h01", "hg080401_h04"],
    },
    Row {
        flag: 3264,
        layers: &["hg010301"],
    },
    Row {
        flag: 3265,
        layers: &["hg010301", "hg010302"],
    },
    Row {
        flag: 3266,
        layers: &["hg020001"],
    },
    Row {
        flag: 3267,
        layers: &["hg020001", "hg020001_h01"],
    },
    Row {
        flag: 3268,
        layers: &["hg020001", "hg020001_h02"],
    },
    Row {
        flag: 3269,
        layers: &["hg040101_h01"],
    },
    Row {
        flag: 3270,
        layers: &["hg040101_h01", "hg040101_h02"],
    },
    Row {
        flag: 3271,
        layers: &["hg040101_h01", "hg040101_h05"],
    },
    Row {
        flag: 3272,
        layers: &["hg040102_h02"],
    },
    Row {
        flag: 3273,
        layers: &["hg040103_h01", "hg040102_h03"],
    },
    Row {
        flag: 3274,
        layers: &["hg040103_h01"],
    },
    Row {
        flag: 3275,
        layers: &["hg040103_h01", "hg040102_h05"],
    },
    Row {
        flag: 3276,
        layers: &["hg040103_h01", "hg040102_h06"],
    },
    Row {
        flag: 3277,
        layers: &["hg040103_h01", "hg040102_h07"],
    },
    Row {
        flag: 3278,
        layers: &["hg040104_h04"],
    },
    Row {
        flag: 3279,
        layers: &["hg040104_h04", "hg040102_h05"],
    },
    Row {
        flag: 3280,
        layers: &["hg040104_h04", "hg040102_h06"],
    },
    Row {
        flag: 3281,
        layers: &["hg040201_h01"],
    },
    Row {
        flag: 3282,
        layers: &["hg040201_h01", "hg040201_h02"],
    },
    Row {
        flag: 3283,
        layers: &["hg040201_h01", "hg040201_h03"],
    },
    Row {
        flag: 3284,
        layers: &["hg040202_h01"],
    },
    Row {
        flag: 3285,
        layers: &["hg040202_h01", "hg040201_h04"],
    },
    Row {
        flag: 3286,
        layers: &["hg050000_h01"],
    },
    Row {
        flag: 3287,
        layers: &["hg050000_h01", "hg050000_h02"],
    },
    Row {
        flag: 3288,
        layers: &["hg030001_h01"],
    },
    Row {
        flag: 3289,
        layers: &["hg030001_h01", "hg030001_h02", "hg030001_h05"],
    },
    Row {
        flag: 3290,
        layers: &["hg030001_h01", "hg030001_h05"],
    },
    Row {
        flag: 3291,
        layers: &["hg030001_h01", "hg030001_h06"],
    },
    Row {
        flag: 3292,
        layers: &["hg030001_h01", "hg030001_h06", "hg030001_h03"],
    },
    Row {
        flag: 3293,
        layers: &["hg030001_h01", "hg030001_h06", "hg030001_h02"],
    },
    Row {
        flag: 3294,
        layers: &["hg090201"],
    },
    Row {
        flag: 3295,
        layers: &["ojisan01"],
    },
    Row {
        flag: 3296,
        layers: &["ojisan02"],
    },
    Row {
        flag: 3297,
        layers: &["ojisan03"],
    },
    Row {
        flag: 3298,
        layers: &["ojisan04"],
    },
    Row {
        flag: 3299,
        layers: &["ojisan05"],
    },
    Row {
        flag: 3300,
        layers: &["ojisan06"],
    },
    Row {
        flag: 3301,
        layers: &["hg050101_h01"],
    },
    Row {
        flag: 3302,
        layers: &["hg050101_h01", "hg050101_h02"],
    },
    Row {
        flag: 3303,
        layers: &["hg050102_h05", "hg050101_h03"],
    },
    Row {
        flag: 3304,
        layers: &["hg050102_h05", "hg050101_h05"],
    },
    Row {
        flag: 3305,
        layers: &["hg050102_h05", "hg050101_h02"],
    },
    Row {
        flag: 3306,
        layers: &["hg050102_h05", "hg050101_h04"],
    },
    Row {
        flag: 3307,
        layers: &["hg050104_h05", "hg050104_h03"],
    },
    Row {
        flag: 3308,
        layers: &["hg050104_h05"],
    },
    Row {
        flag: 3309,
        layers: &["hg050104_h05", "hg050104_h04"],
    },
    Row {
        flag: 3310,
        layers: &["hg050201_h01"],
    },
    Row {
        flag: 3311,
        layers: &["hg050201_h01", "hg050201_h02"],
    },
    Row {
        flag: 3312,
        layers: &["hg050201_h01", "hg050201_h03"],
    },
    Row {
        flag: 3313,
        layers: &["hg050201_h01", "hg050201_h04"],
    },
    Row {
        flag: 3314,
        layers: &["hg050202_h01", "hg050202_h03"],
    },
    Row {
        flag: 3315,
        layers: &["hg050202_h01"],
    },
    Row {
        flag: 3316,
        layers: &["hg050202_h01", "hg050202_h02"],
    },
    Row {
        flag: 3317,
        layers: &["hg050202_h01", "hg050202_h04"],
    },
    Row {
        flag: 3318,
        layers: &["hg050203_h02", "hg050203_h03"],
    },
    Row {
        flag: 3319,
        layers: &["hg050203_h02", "hg050203_h04"],
    },
    Row {
        flag: 3320,
        layers: &["hg050203_h02"],
    },
    Row {
        flag: 3321,
        layers: &["hg060003"],
    },
    Row {
        flag: 3322,
        layers: &["hg060004"],
    },
    Row {
        flag: 3323,
        layers: &["hg060005"],
    },
    Row {
        flag: 3324,
        layers: &["hg060001_h02"],
    },
    Row {
        flag: 3325,
        layers: &["hg060001_h01"],
    },
    Row {
        flag: 3326,
        layers: &["hg060002_h01"],
    },
    Row {
        flag: 3327,
        layers: &["hg020101"],
    },
    Row {
        flag: 3328,
        layers: &["hg020101", "hg020101_h01"],
    },
    Row {
        flag: 3329,
        layers: &["hg020101", "hg020101_h04"],
    },
    Row {
        flag: 3330,
        layers: &["hg020101", "hg020101_h04", "hg020101_h02"],
    },
    Row {
        flag: 3331,
        layers: &["hg020101", "hg020101_h04", "hg020101_h03"],
    },
    Row {
        flag: 3332,
        layers: &["hg020101", "hg020101_h03"],
    },
    Row {
        flag: 3333,
        layers: &["hg090300"],
    },
    Row {
        flag: 3334,
        layers: &["hg090301"],
    },
    Row {
        flag: 3335,
        layers: &["hg090302"],
    },
    Row {
        flag: 3336,
        layers: &["hg010500_h01"],
    },
    Row {
        flag: 3337,
        layers: &["hg010500_h01", "hg010500_h03"],
    },
    Row {
        flag: 3338,
        layers: &["hg010500_h01", "hg010500_h02"],
    },
    Row {
        flag: 3339,
        layers: &["hg010500_h01", "hg010500_h04"],
    },
    Row {
        flag: 3340,
        layers: &["hg010500_h01", "hg010500_h05"],
    },
    Row {
        flag: 3341,
        layers: &["hg030101_h01"],
    },
    Row {
        flag: 3342,
        layers: &["hg030101_h01", "hg030101_h02"],
    },
    Row {
        flag: 3343,
        layers: &["hg030103_h01", "hg030101_h02"],
    },
    Row {
        flag: 3344,
        layers: &["hg030103_h01"],
    },
    Row {
        flag: 3345,
        layers: &["hg030103_h01", "hg030101_h03"],
    },
    Row {
        flag: 3346,
        layers: &["hg030104_h01", "hg030101_h03"],
    },
    Row {
        flag: 3347,
        layers: &["hg030104_h01"],
    },
    Row {
        flag: 3348,
        layers: &["hg030105_h01", "hg030101_h03"],
    },
    Row {
        flag: 3349,
        layers: &["hg030105_h01", "hg030101_h04"],
    },
    Row {
        flag: 3350,
        layers: &["hg030105_h01", "hg030101_h02"],
    },
    Row {
        flag: 3351,
        layers: &["hg030106_h01", "hg030101_h04"],
    },
    Row {
        flag: 3352,
        layers: &["hg030106_h01", "hg030101_h03"],
    },
    Row {
        flag: 3353,
        layers: &["hg030201_h01"],
    },
    Row {
        flag: 3354,
        layers: &["hg030201_h01", "hg030201_h02"],
    },
    Row {
        flag: 3355,
        layers: &["hg030201_h01", "hg030201_h03"],
    },
    Row {
        flag: 3356,
        layers: &["hg030202_h04", "hg030201_h02"],
    },
    Row {
        flag: 3357,
        layers: &["hg030202_h04"],
    },
    Row {
        flag: 3358,
        layers: &["hg070101_h01"],
    },
    Row {
        flag: 3359,
        layers: &["hg070101_h01", "hg070101_h02"],
    },
    Row {
        flag: 3360,
        layers: &["hg070101_h01", "hg070101_h03"],
    },
    Row {
        flag: 3361,
        layers: &["hg070101_h01", "hg070101_h04"],
    },
    Row {
        flag: 3362,
        layers: &["hg070201_h01", "hg070201_h02"],
    },
    Row {
        flag: 3363,
        layers: &["hg070201_h01"],
    },
    Row {
        flag: 3364,
        layers: &["hg070201_h01", "hg070201_h03"],
    },
    Row {
        flag: 3365,
        layers: &["hg070201_h01", "hg070201_h04"],
    },
    Row {
        flag: 3366,
        layers: &["hg070201_h01", "hg070201_h05"],
    },
    Row {
        flag: 3367,
        layers: &["hg070202"],
    },
    Row {
        flag: 3368,
        layers: &["hg070202", "hg070201_h06"],
    },
    Row {
        flag: 3369,
        layers: &["hg090401_h01"],
    },
    Row {
        flag: 3370,
        layers: &["hg090401_h01", "hg090401_h05"],
    },
    Row {
        flag: 3371,
        layers: &["hg090401_h01", "hg090401_h05", "hg090401_h02"],
    },
    Row {
        flag: 3372,
        layers: &[
            "hg090401_h01",
            "hg090401_h05",
            "hg090401_h02",
            "hg090401_h08",
        ],
    },
    Row {
        flag: 3373,
        layers: &[
            "hg090401_h01",
            "hg090401_h06",
            "hg090401_h02",
            "hg090401_h08",
        ],
    },
    Row {
        flag: 3374,
        layers: &[
            "hg090401_h01",
            "hg090401_h06",
            "hg090401_h03",
            "hg090401_h08",
        ],
    },
    Row {
        flag: 3375,
        layers: &[
            "hg090401_h01",
            "hg090401_h06",
            "hg090401_h03",
            "hg090401_h09",
        ],
    },
    Row {
        flag: 3376,
        layers: &[
            "hg090401_h01",
            "hg090401_h06",
            "hg090401_h02",
            "hg090401_h09",
        ],
    },
    Row {
        flag: 3377,
        layers: &[
            "hg090401_h01",
            "hg090401_h07",
            "hg090401_h04",
            "hg090401_h10",
        ],
    },
    Row {
        flag: 3378,
        layers: &["hg090402_h04"],
    },
    Row {
        flag: 3379,
        layers: &["hg090501_h01"],
    },
    Row {
        flag: 3380,
        layers: &["hg090501_h01", "hg090500_h08"],
    },
    Row {
        flag: 3381,
        layers: &["hg090502_h01", "hg090500_h05"],
    },
    Row {
        flag: 3382,
        layers: &["hg090502_h01", "hg090500_h06"],
    },
    Row {
        flag: 3383,
        layers: &["hg090503_h01", "hg090500_h07"],
    },
    Row {
        flag: 3384,
        layers: &["hg090504_h01", "hg090500_h07"],
    },
    Row {
        flag: 3385,
        layers: &["hg090504_h01", "hg090500_h07", "hg090500_h08"],
    },
    Row {
        flag: 3386,
        layers: &[
            "hg090505_h01",
            "hg090500_h07",
            "hg090500_h08",
            "hg090500_h02",
        ],
    },
    Row {
        flag: 3387,
        layers: &["hg090505_h01", "hg090500_h07", "hg090500_h02"],
    },
    Row {
        flag: 3388,
        layers: &["hg090505_h01", "hg090500_h07", "hg090500_h03"],
    },
    Row {
        flag: 3389,
        layers: &["hg090506_h01", "hg090500_h07", "hg090500_h04"],
    },
    Row {
        flag: 3390,
        layers: &[
            "hg090506_h01",
            "hg090500_h07",
            "hg090500_h08",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3391,
        layers: &[
            "hg090507_h01",
            "hg090500_h07",
            "hg090500_h08",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3392,
        layers: &["hg090507_h01", "hg090500_h07", "hg090500_h04"],
    },
    Row {
        flag: 3393,
        layers: &[
            "hg090508_h01",
            "hg090500_h07",
            "hg090500_h09",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3394,
        layers: &["hg090508_h01", "hg090500_h07", "hg090500_h04"],
    },
    Row {
        flag: 3395,
        layers: &[
            "hg090508_h01",
            "hg090500_h07",
            "hg090500_h08",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3396,
        layers: &[
            "hg090508_h01",
            "hg090500_h07",
            "hg090500_h10",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3397,
        layers: &[
            "hg090509_h01",
            "hg090500_h07",
            "hg090500_h10",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3398,
        layers: &[
            "hg090509_h01",
            "hg090500_h07",
            "hg090500_h09",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3399,
        layers: &[
            "hg090510_h01",
            "hg090500_h07",
            "hg090500_h10",
            "hg090500_h04",
        ],
    },
    Row {
        flag: 3400,
        layers: &["hg060101_h01"],
    },
    Row {
        flag: 3401,
        layers: &["hg060102_h01"],
    },
    Row {
        flag: 3402,
        layers: &["hg060103_h01", "hg060103_h02"],
    },
    Row {
        flag: 3403,
        layers: &["hg060103_h01"],
    },
    Row {
        flag: 3404,
        layers: &["hg060103_h01", "hg060103_h03"],
    },
    Row {
        flag: 3405,
        layers: &["hg060103_h01", "hg060103_h04"],
    },
    Row {
        flag: 3406,
        layers: &["hg060104_h04"],
    },
    Row {
        flag: 3407,
        layers: &["hg060104_h04", "hg060103_h03"],
    },
    Row {
        flag: 3408,
        layers: &["hg020201_h01"],
    },
    Row {
        flag: 3409,
        layers: &["hg020201_h01", "hg020201_h02"],
    },
    Row {
        flag: 3410,
        layers: &["hg020202_h01"],
    },
    Row {
        flag: 3411,
        layers: &["hg020202_h01", "hg020201_h03"],
    },
    Row {
        flag: 3412,
        layers: &["hg020203_h01", "hg020203_h03"],
    },
    Row {
        flag: 3413,
        layers: &["hg020203_h01"],
    },
    Row {
        flag: 3414,
        layers: &["hg020204_h01", "hg020203_h04"],
    },
    Row {
        flag: 3415,
        layers: &["hg020204_h01", "hg020203_h03"],
    },
    Row {
        flag: 3416,
        layers: &["hg020204_h01"],
    },
    Row {
        flag: 3417,
        layers: &["hg020204_h01", "hg020203_h05"],
    },
    Row {
        flag: 3418,
        layers: &["hg020204_h01", "hg020203_h02"],
    },
    Row {
        flag: 3419,
        layers: &["hg020206_h01", "hg020206_h06"],
    },
    Row {
        flag: 3420,
        layers: &["hg020207_h01", "hg020206_h06"],
    },
    Row {
        flag: 3421,
        layers: &["hg020207_h01", "hg020206_h07"],
    },
    Row {
        flag: 3422,
        layers: &["hg020208_h01", "hg020206_h07"],
    },
    Row {
        flag: 3423,
        layers: &["hg020209_h01"],
    },
    Row {
        flag: 3424,
        layers: &["hg020209_h01", "hg020206_h05"],
    },
    Row {
        flag: 3425,
        layers: &["hg020209_h01", "hg020206_h07"],
    },
    Row {
        flag: 3426,
        layers: &["hg020210_h01", "hg020206_h04"],
    },
    Row {
        flag: 3427,
        layers: &["hg020210_h01", "hg020206_h06"],
    },
    Row {
        flag: 3428,
        layers: &["hg020211_h01", "hg020206_h07"],
    },
    Row {
        flag: 3429,
        layers: &["hg020211_h01"],
    },
    Row {
        flag: 3430,
        layers: &["hg020212_h01"],
    },
    Row {
        flag: 3431,
        layers: &["hg020212_h01", "hg020206_h05"],
    },
    Row {
        flag: 3432,
        layers: &["hg020212_h01", "hg020206_h02"],
    },
    Row {
        flag: 3433,
        layers: &["hg020212_h01", "hg020206_h03"],
    },
    Row {
        flag: 3434,
        layers: &["hg020213_h01", "hg020206_h08"],
    },
    Row {
        flag: 3435,
        layers: &["hg020213_h01", "hg020206_h07"],
    },
    Row {
        flag: 3436,
        layers: &["hg020213_h01", "hg020206_h06"],
    },
    Row {
        flag: 3437,
        layers: &["hg020213_h01", "hg020206_h04"],
    },
    Row {
        flag: 3438,
        layers: &["hg020213_h01", "hg020206_h03"],
    },
    Row {
        flag: 3439,
        layers: &["hg060201_h01"],
    },
    Row {
        flag: 3440,
        layers: &["hg060201_h01", "hg060201_h03"],
    },
    Row {
        flag: 3441,
        layers: &["hg060201_h01", "hg060201_h02"],
    },
    Row {
        flag: 3442,
        layers: &["hg060201_h01", "hg060201_h04"],
    },
    Row {
        flag: 3443,
        layers: &["hg060202_h04", "hg060201_h03"],
    },
    Row {
        flag: 3444,
        layers: &["hg060202_h04"],
    },
    Row {
        flag: 3445,
        layers: &["hg060301_h01"],
    },
    Row {
        flag: 3446,
        layers: &["hg060302_h02"],
    },
    Row {
        flag: 3447,
        layers: &["hg060302_h02", "hg060301_h03"],
    },
    Row {
        flag: 3448,
        layers: &["hg060303_h03"],
    },
    Row {
        flag: 3449,
        layers: &["hg060303_h03", "hg060301_h04"],
    },
    Row {
        flag: 3450,
        layers: &["hg060303_h03", "hg060301_h05"],
    },
    Row {
        flag: 3451,
        layers: &["hg060304_h05", "hg060301_h05"],
    },
    Row {
        flag: 3452,
        layers: &["hg060304_h05", "hg060301_h04"],
    },
    Row {
        flag: 3453,
        layers: &["hg060304_h05", "hg060301_h06"],
    },
    Row {
        flag: 3454,
        layers: &["hg050301_h01"],
    },
    Row {
        flag: 3455,
        layers: &["hg050301_h01", "hg050301_h03"],
    },
    Row {
        flag: 3456,
        layers: &["hg050301_h01", "hg050301_h02"],
    },
    Row {
        flag: 3457,
        layers: &["hg050301_h01", "hg050301_h04"],
    },
    Row {
        flag: 3458,
        layers: &["hg050302_h01", "hg050301_h03"],
    },
    Row {
        flag: 3459,
        layers: &["hg050302_h01"],
    },
    Row {
        flag: 3460,
        layers: &["hg050302_h01", "hg050301_h02"],
    },
    Row {
        flag: 3461,
        layers: &["hg050302_h01", "hg050301_h04"],
    },
    Row {
        flag: 3462,
        layers: &["hg050303_h01", "hg050301_h03"],
    },
    Row {
        flag: 3463,
        layers: &["hg050303_h01"],
    },
    Row {
        flag: 3464,
        layers: &["hg050303_h01", "hg050301_h02"],
    },
    Row {
        flag: 3465,
        layers: &["hg050303_h01", "hg050301_h05"],
    },
    Row {
        flag: 3466,
        layers: &["hg050304_h01", "hg050301_h05"],
    },
    Row {
        flag: 3467,
        layers: &["hg050304_h01"],
    },
    Row {
        flag: 3468,
        layers: &["hg050304_h01", "hg050301_h04"],
    },
    Row {
        flag: 3469,
        layers: &["hg020301_h01"],
    },
    Row {
        flag: 3470,
        layers: &["hg020301_h01", "hg020301_h02"],
    },
    Row {
        flag: 3471,
        layers: &["hg020302_h01", "hg020301_h02"],
    },
    Row {
        flag: 3472,
        layers: &["hg020302_h01"],
    },
    Row {
        flag: 3473,
        layers: &["hg020302_h01", "hg020301_h03"],
    },
    Row {
        flag: 3474,
        layers: &["hg020302_h01", "hg020301_h04"],
    },
    Row {
        flag: 3475,
        layers: &["hg020302_h01", "hg020301_h05"],
    },
    Row {
        flag: 3476,
        layers: &["hg020303_h01", "hg020301_h05"],
    },
    Row {
        flag: 3477,
        layers: &["hg020303_h01", "hg020301_h06"],
    },
    Row {
        flag: 3478,
        layers: &["hg030301_h01"],
    },
    Row {
        flag: 3479,
        layers: &["hg030301_h01", "hg030301_h02"],
    },
    Row {
        flag: 3480,
        layers: &["hg030301_h01", "hg030301_h03"],
    },
    Row {
        flag: 3481,
        layers: &["hg030301_h01", "hg030301_h04"],
    },
    Row {
        flag: 3482,
        layers: &["hg030301_h01", "hg030301_h05"],
    },
    Row {
        flag: 3483,
        layers: &["hg030401_h01"],
    },
    Row {
        flag: 3484,
        layers: &["hg030401_h01", "hg030401_h02"],
    },
    Row {
        flag: 3485,
        layers: &["hg030402", "hg030401_h02"],
    },
    Row {
        flag: 3486,
        layers: &["hg030402", "hg030401_h03"],
    },
    Row {
        flag: 3487,
        layers: &["hg030402"],
    },
    Row {
        flag: 3488,
        layers: &["hg030403", "hg030401_h03"],
    },
    Row {
        flag: 3489,
        layers: &["hg030403", "hg030401_h02"],
    },
    Row {
        flag: 3490,
        layers: &["hg030403", "hg030401_h04"],
    },
    Row {
        flag: 3491,
        layers: &["hg030403"],
    },
    Row {
        flag: 3492,
        layers: &["hg030404", "hg030401_h03"],
    },
    Row {
        flag: 3493,
        layers: &["hg030404", "hg030401_h04"],
    },
    Row {
        flag: 3494,
        layers: &["hg030404", "hg030401_h05"],
    },
    Row {
        flag: 3495,
        layers: &["hg030404", "hg030401_h02"],
    },
    Row {
        flag: 3496,
        layers: &["hg030405", "hg030401_h05"],
    },
    Row {
        flag: 3497,
        layers: &["hg030405", "hg030401_h04"],
    },
    Row {
        flag: 3498,
        layers: &["hg020401_h01"],
    },
    Row {
        flag: 3499,
        layers: &["hg020401_h01", "hg020401_h02"],
    },
    Row {
        flag: 3500,
        layers: &["hg020401_h01", "hg020401_h03"],
    },
    Row {
        flag: 3501,
        layers: &["hg020402_h01", "hg020401_h03"],
    },
    Row {
        flag: 3502,
        layers: &["hg020402_h01"],
    },
    Row {
        flag: 3503,
        layers: &["hg020402_h01", "hg020401_h02"],
    },
    Row {
        flag: 3504,
        layers: &["hg020403_h01", "hg020401_h04"],
    },
    Row {
        flag: 3505,
        layers: &["hg020403_h01", "hg020401_h03"],
    },
    Row {
        flag: 3506,
        layers: &["hg020403_h01", "hg020401_h02"],
    },
    Row {
        flag: 3507,
        layers: &["hg040301_h01"],
    },
    Row {
        flag: 3508,
        layers: &["hg040301_h01", "hg040301_h02"],
    },
    Row {
        flag: 3509,
        layers: &["hg040301_h01", "hg040301_h04"],
    },
    Row {
        flag: 3510,
        layers: &["hg040304_h03"],
    },
    Row {
        flag: 3511,
        layers: &["hg040304_h03", "hg040304_h05"],
    },
    Row {
        flag: 3512,
        layers: &["hg040304_h03", "hg040304_h06"],
    },
    Row {
        flag: 3513,
        layers: &["hg040302_h01"],
    },
    Row {
        flag: 3514,
        layers: &["hg040305_h03"],
    },
    Row {
        flag: 3515,
        layers: &["hg040305_h03", "hg040304_h05"],
    },
    Row {
        flag: 3516,
        layers: &["hg040303_h01"],
    },
    Row {
        flag: 3517,
        layers: &["hg020500_h01"],
    },
    Row {
        flag: 3518,
        layers: &["hg020500_h01", "hg020500_h02"],
    },
    Row {
        flag: 3519,
        layers: &["hg020500_h01", "hg020500_h03"],
    },
    Row {
        flag: 3520,
        layers: &["hg020500_h01", "hg020500_h05"],
    },
    Row {
        flag: 3521,
        layers: &["hg020500_h01", "hg020500_h04"],
    },
    Row {
        flag: 3522,
        layers: &["hg020501_h01"],
    },
    Row {
        flag: 3523,
        layers: &["hg020501_h01", "hg020501_h02"],
    },
    Row {
        flag: 3524,
        layers: &["hg020501_h01", "hg020501_h03"],
    },
    Row {
        flag: 3525,
        layers: &["hg020501_h01", "hg020501_h04"],
    },
    Row {
        flag: 3526,
        layers: &["hg020501_h01", "hg020501_h05"],
    },
    Row {
        flag: 3527,
        layers: &["hg020502_h01", "hg020502_h03"],
    },
    Row {
        flag: 3528,
        layers: &["hg020502_h01", "hg020502_h02"],
    },
    Row {
        flag: 3529,
        layers: &["hg020502_h01", "hg020502_h04"],
    },
    Row {
        flag: 3530,
        layers: &["hg020502_h01", "hg020502_h05"],
    },
    Row {
        flag: 3531,
        layers: &["hg010601_h01"],
    },
    Row {
        flag: 3532,
        layers: &["hg010601_h01", "hg010601_h02"],
    },
    Row {
        flag: 3533,
        layers: &["hg010602_h02"],
    },
    Row {
        flag: 3534,
        layers: &["hg010603"],
    },
    Row {
        flag: 3535,
        layers: &["hg090601_h01"],
    },
    Row {
        flag: 3536,
        layers: &["hg010701_h01", "hg010701_h05"],
    },
    Row {
        flag: 3537,
        layers: &["hg010701_h01", "hg010701_h05", "hg010701_h03"],
    },
    Row {
        flag: 3538,
        layers: &["hg010701_h01", "hg010701_h05", "hg010701_h04"],
    },
    Row {
        flag: 3539,
        layers: &["hg010701_h01", "hg010701_h05", "hg010701_h02"],
    },
    Row {
        flag: 3540,
        layers: &["hg010701_h01", "hg010701_h06", "hg010701_h02"],
    },
    Row {
        flag: 3541,
        layers: &["hg010701_h01", "hg010701_h06", "hg010701_h03"],
    },
    Row {
        flag: 3542,
        layers: &["hg010702", "hg010701_h06"],
    },
    Row {
        flag: 3543,
        layers: &["hg010702", "hg010701_h03"],
    },
    Row {
        flag: 3544,
        layers: &["hg010702", "hg010701_h03", "hg010701_h06"],
    },
    Row {
        flag: 3545,
        layers: &["hg010702", "hg010701_h04", "hg010701_h05"],
    },
    Row {
        flag: 3546,
        layers: &["hg010703", "hg010701_h03", "hg010701_h05"],
    },
    Row {
        flag: 3547,
        layers: &["hg010704", "hg010701_h03"],
    },
    Row {
        flag: 3548,
        layers: &["hg010704", "hg010701_h03", "hg010701_h06"],
    },
    Row {
        flag: 3549,
        layers: &["hg010704", "hg010701_h02", "hg010701_h06"],
    },
    Row {
        flag: 3550,
        layers: &["hg010704", "hg010701_h04", "hg010701_h05"],
    },
    Row {
        flag: 3551,
        layers: &["hg010701_h01", "hg010701_h02", "hg010701_h06"],
    },
    Row {
        flag: 3552,
        layers: &["hg010701_h01", "hg010701_h02"],
    },
    Row {
        flag: 3553,
        layers: &["hg010703", "hg010701_h02", "hg010701_h06"],
    },
    Row {
        flag: 3554,
        layers: &["hg010703", "hg010701_h03"],
    },
    Row {
        flag: 3555,
        layers: &["hg010703"],
    },
    Row {
        flag: 3556,
        layers: &["hg010801_h01"],
    },
    Row {
        flag: 3557,
        layers: &["hg010801_h01", "hg010801_h02"],
    },
    Row {
        flag: 3558,
        layers: &["hg010801_h01", "hg010801_h03"],
    },
    Row {
        flag: 3559,
        layers: &["hg010802_h01"],
    },
    Row {
        flag: 3560,
        layers: &["hg010802_h01", "hg010801_h02"],
    },
    Row {
        flag: 3561,
        layers: &["hg010803_h01"],
    },
    Row {
        flag: 3562,
        layers: &["hg010803_h01", "hg010801_h03"],
    },
    Row {
        flag: 3563,
        layers: &["hg010803_h01", "hg010801_h02"],
    },
    Row {
        flag: 3564,
        layers: &["hg090711_h01"],
    },
    Row {
        flag: 3565,
        layers: &["hg090711_h01", "hg090711_h02"],
    },
    Row {
        flag: 3566,
        layers: &["hg090711_h01", "hg090711_h03"],
    },
    Row {
        flag: 3567,
        layers: &["hg090712", "hg090711_h04"],
    },
    Row {
        flag: 3568,
        layers: &["hg090713", "hg090711_h04"],
    },
    Row {
        flag: 3569,
        layers: &["hg090714", "hg090711_h04"],
    },
    Row {
        flag: 3570,
        layers: &["hg090714", "hg090711_h02"],
    },
    Row {
        flag: 3571,
        layers: &["hg090714"],
    },
    Row {
        flag: 3572,
        layers: &["hg090714", "hg090711_h05"],
    },
    Row {
        flag: 3573,
        layers: &["hg090715"],
    },
    Row {
        flag: 3574,
        layers: &["hg090721_h06"],
    },
    Row {
        flag: 3575,
        layers: &["hg090721_h06", "hg090721_h07"],
    },
    Row {
        flag: 3576,
        layers: &["hg090721_h06", "hg090721_h08"],
    },
    Row {
        flag: 3577,
        layers: &["hg090725"],
    },
    Row {
        flag: 3578,
        layers: &["hg090726"],
    },
    Row {
        flag: 3579,
        layers: &["hg091201_h01", "hg091201_h02"],
    },
    Row {
        flag: 3580,
        layers: &["hg091201_h01"],
    },
    Row {
        flag: 3581,
        layers: &["hg091201_h01", "hg091201_h03"],
    },
    Row {
        flag: 3582,
        layers: &["hg091201_h01", "hg091201_h05"],
    },
    Row {
        flag: 3583,
        layers: &["hg091202_h01"],
    },
    Row {
        flag: 3584,
        layers: &["hg091202_h01", "hg091202_h03"],
    },
    Row {
        flag: 3585,
        layers: &["hg091202_h01", "hg091202_h02"],
    },
    Row {
        flag: 3586,
        layers: &["hg091202_h01", "hg091202_h06"],
    },
    Row {
        flag: 3587,
        layers: &["hg091202_h01", "hg091202_h04"],
    },
    Row {
        flag: 3588,
        layers: &["hg091202_h01", "hg091202_h05"],
    },
    Row {
        flag: 3589,
        layers: &["hg091202_h01", "hg091202_h07"],
    },
    Row {
        flag: 3590,
        layers: &["hg090801"],
    },
    Row {
        flag: 3591,
        layers: &["hg090800"],
    },
    Row {
        flag: 3592,
        layers: &["hg090903"],
    },
    Row {
        flag: 3593,
        layers: &["hg090900"],
    },
    Row {
        flag: 3594,
        layers: &["hg090901"],
    },
    Row {
        flag: 3595,
        layers: &["hg010901_h01"],
    },
    Row {
        flag: 3596,
        layers: &["hg010901_h01", "hg010901_h02"],
    },
    Row {
        flag: 3597,
        layers: &["hg010903_h01", "hg010901_h03"],
    },
    Row {
        flag: 3598,
        layers: &["hg010903_h01"],
    },
    Row {
        flag: 3599,
        layers: &["hg010902_h01", "hg010901_h02"],
    },
    Row {
        flag: 3600,
        layers: &["hg010902_h01", "hg010901_h03"],
    },
    Row {
        flag: 3601,
        layers: &["hg010902_h01", "hg010901_h04"],
    },
    Row {
        flag: 3602,
        layers: &["hg010902_h01", "hg010901_h05"],
    },
    Row {
        flag: 3603,
        layers: &["hg010904_h01", "hg010901_h05"],
    },
    Row {
        flag: 3604,
        layers: &["hg011001_h01"],
    },
    Row {
        flag: 3605,
        layers: &["hg011001_h01", "hg011001_h03", "hg011001_h08"],
    },
    Row {
        flag: 3606,
        layers: &["hg011001_h01", "hg011001_h02", "hg011001_h07"],
    },
    Row {
        flag: 3607,
        layers: &["hg011001_h01", "hg011001_h08"],
    },
    Row {
        flag: 3608,
        layers: &["hg011004_h01", "hg011001_h04"],
    },
    Row {
        flag: 3609,
        layers: &["hg011004_h01", "hg011001_h05"],
    },
    Row {
        flag: 3610,
        layers: &["hg011004_h01"],
    },
    Row {
        flag: 3611,
        layers: &["hg011004_h01", "hg011001_h06"],
    },
    Row {
        flag: 3612,
        layers: &["hg011002_h04", "hg011001_h06"],
    },
    Row {
        flag: 3613,
        layers: &["hg011003_h05"],
    },
    Row {
        flag: 3614,
        layers: &["hg011003_h05", "hg011003_h06"],
    },
    Row {
        flag: 3615,
        layers: &["hg011003_h05", "hg011001_h02", "hg011001_h07"],
    },
    Row {
        flag: 3616,
        layers: &["hg091101"],
    },
    Row {
        flag: 3617,
        layers: &["hg091102"],
    },
    Row {
        flag: 3618,
        layers: &["hg091103"],
    },
    Row {
        flag: 3619,
        layers: &["hg091104"],
    },
    Row {
        flag: 3620,
        layers: &["hg091105"],
    },
    Row {
        flag: 3621,
        layers: &["hg091106"],
    },
    Row {
        flag: 3622,
        layers: &["hg091107"],
    },
    Row {
        flag: 3623,
        layers: &["hg091108"],
    },
    Row {
        flag: 3624,
        layers: &["hg091109"],
    },
    Row {
        flag: 3625,
        layers: &["hg011201_h01"],
    },
    Row {
        flag: 3626,
        layers: &["hg011201_h01", "hg011201_h02"],
    },
    Row {
        flag: 3627,
        layers: &["hg011206_h01", "hg011201_h02"],
    },
    Row {
        flag: 3628,
        layers: &["hg011206_h01", "hg011201_h03"],
    },
    Row {
        flag: 3629,
        layers: &["hg011202_h01", "hg011201_h03"],
    },
    Row {
        flag: 3630,
        layers: &["hg011203_h01", "hg011201_h03"],
    },
    Row {
        flag: 3631,
        layers: &["hg011207_h01", "hg011201_h03"],
    },
    Row {
        flag: 3632,
        layers: &["hg011204_h03"],
    },
    Row {
        flag: 3633,
        layers: &["hg011205_h01"],
    },
    Row {
        flag: 3634,
        layers: &["hg091001_h01_00"],
    },
    Row {
        flag: 3635,
        layers: &["hg091001_h01_01"],
    },
    Row {
        flag: 3636,
        layers: &["hg091001_h01"],
    },
    Row {
        flag: 3637,
        layers: &["hg091002_h01"],
    },
    Row {
        flag: 3638,
        layers: &["hg091003_h01"],
    },
    Row {
        flag: 3639,
        layers: &["hg091003_h01", "hg091010_h03"],
    },
    Row {
        flag: 3640,
        layers: &["hg091004_h01"],
    },
    Row {
        flag: 3641,
        layers: &["hg091004_h01", "hg091010_h03"],
    },
    Row {
        flag: 3642,
        layers: &["hg091005_h01"],
    },
    Row {
        flag: 3643,
        layers: &["hg091006_h01"],
    },
    Row {
        flag: 3644,
        layers: &["hg091007_h01"],
    },
    Row {
        flag: 3645,
        layers: &["hg091007_h01", "hg091010_h02"],
    },
    Row {
        flag: 3646,
        layers: &["hg091008_h01"],
    },
    Row {
        flag: 3647,
        layers: &["hg091009_h01"],
    },
    Row {
        flag: 3648,
        layers: &["hg091010_h01"],
    },
    Row {
        flag: 3649,
        layers: &["hg091010_h01", "hg091010_h05"],
    },
    Row {
        flag: 3650,
        layers: &["hg091010_h01", "hg091010_h05", "hg091010_h04"],
    },
    Row {
        flag: 3651,
        layers: &["hg091011_h01"],
    },
    Row {
        flag: 3652,
        layers: &["hg091012_h02"],
    },
    Row {
        flag: 3653,
        layers: &["hg091012_h02", "hg091012_h04"],
    },
    Row {
        flag: 3654,
        layers: &["hg091013_h02", "hg091013_h04"],
    },
    Row {
        flag: 3655,
        layers: &["hg091014_h02"],
    },
    Row {
        flag: 3656,
        layers: &["hg091014_h02", "hg091012_h03"],
    },
    Row {
        flag: 3657,
        layers: &["hg070301_h01"],
    },
    Row {
        flag: 3658,
        layers: &["hg070301_h01", "hg070301_h02"],
    },
    Row {
        flag: 3659,
        layers: &["hg070301_h01", "hg070301_h03"],
    },
    Row {
        flag: 3660,
        layers: &["hg070301_h01", "hg070301_h06"],
    },
    Row {
        flag: 3661,
        layers: &["hg070301_h01", "hg070301_h04"],
    },
    Row {
        flag: 3662,
        layers: &["hg070301_h01", "hg070301_h05"],
    },
    Row {
        flag: 3663,
        layers: &["hg070302_h01", "hg070301_h05"],
    },
    Row {
        flag: 3664,
        layers: &["hg070302_h01", "hg070301_h04"],
    },
    Row {
        flag: 3665,
        layers: &["hg070302_h01", "hg070301_h03"],
    },
    Row {
        flag: 3666,
        layers: &["hg070302_h01", "hg070301_h06"],
    },
    Row {
        flag: 3667,
        layers: &["hg070302_h01"],
    },
    Row {
        flag: 3668,
        layers: &["hg070302_h01", "hg070301_h02"],
    },
    Row {
        flag: 3669,
        layers: &["hg070303_h01", "hg070301_h07"],
    },
    Row {
        flag: 3670,
        layers: &["hg070303_h01", "hg070301_h06"],
    },
    Row {
        flag: 3671,
        layers: &["hg070304_h01", "hg070301_h07"],
    },
    Row {
        flag: 3672,
        layers: &["hg070304_h01", "hg070301_h09"],
    },
    Row {
        flag: 3673,
        layers: &["hg070304_h01", "hg070301_h08"],
    },
    Row {
        flag: 3674,
        layers: &["hg070305_h01", "hg070301_h09"],
    },
    Row {
        flag: 3675,
        layers: &["hg070305_h01", "hg070301_h08"],
    },
    Row {
        flag: 3676,
        layers: &["hg070306_h01", "hg070301_h06"],
    },
    Row {
        flag: 3677,
        layers: &["hg070306_h01", "hg070301_h07"],
    },
    Row {
        flag: 3678,
        layers: &["hg070307_h01", "hg070301_h06"],
    },
    Row {
        flag: 3679,
        layers: &["hg070307_h01", "hg070301_h07"],
    },
    Row {
        flag: 3680,
        layers: &["hg070401_h01"],
    },
    Row {
        flag: 3681,
        layers: &["hg070402_h01", "hg070401_h02"],
    },
    Row {
        flag: 3682,
        layers: &["hg070402_h01"],
    },
    Row {
        flag: 3683,
        layers: &["hg070403_h01", "hg070401_h02"],
    },
    Row {
        flag: 3684,
        layers: &["hg070403_h01", "hg070401_h03"],
    },
    Row {
        flag: 3685,
        layers: &["hg070501", "hg070501_h01"],
    },
    Row {
        flag: 3686,
        layers: &["hg070501", "hg070501_h02"],
    },
    Row {
        flag: 3687,
        layers: &["hg070501", "hg070501_h06"],
    },
    Row {
        flag: 3688,
        layers: &["hg070501", "hg070501_h05"],
    },
    Row {
        flag: 3689,
        layers: &["hg070501"],
    },
    Row {
        flag: 3690,
        layers: &["hg070501", "hg070501_h03"],
    },
    Row {
        flag: 3691,
        layers: &["hg070501", "hg070501_h04"],
    },
    Row {
        flag: 3692,
        layers: &["hg070501", "hg070501_h07"],
    },
    Row {
        flag: 3694,
        layers: &["hg070502"],
    },
    Row {
        flag: 3695,
        layers: &["hg070503"],
    },
    Row {
        flag: 3696,
        layers: &["hg070600"],
    },
    Row {
        flag: 3697,
        layers: &["hg070700"],
    },
    Row {
        flag: 3698,
        layers: &["hg011301_h01"],
    },
    Row {
        flag: 3699,
        layers: &["hg011302_h01", "hg011303_h02"],
    },
    Row {
        flag: 3700,
        layers: &["hg011302_h01"],
    },
    Row {
        flag: 3701,
        layers: &["hg011302_h01", "hg011303_h03"],
    },
    Row {
        flag: 3702,
        layers: &["hg011303_h01", "hg011303_h04"],
    },
    Row {
        flag: 3703,
        layers: &["hg011303_h01", "hg011303_h03"],
    },
    Row {
        flag: 3704,
        layers: &["hg011303_h01", "hg011303_h02"],
    },
    Row {
        flag: 3705,
        layers: &["hg011303_h01", "hg011303_h05"],
    },
    Row {
        flag: 3706,
        layers: &["hg011304_h06", "hg011303_h05"],
    },
    Row {
        flag: 3707,
        layers: &["hg011304_h06", "hg011303_h03"],
    },
    Row {
        flag: 3708,
        layers: &["hg011304_h06"],
    },
    Row {
        flag: 3709,
        layers: &["hg011305_h06", "hg011303_h03"],
    },
    Row {
        flag: 3710,
        layers: &["hg011305_h06", "hg011303_h02"],
    },
    Row {
        flag: 3711,
        layers: &["hg011305_h06"],
    },
    Row {
        flag: 3712,
        layers: &["hg011401_h01", "hg011401_h03"],
    },
    Row {
        flag: 3713,
        layers: &["hg011401_h01"],
    },
    Row {
        flag: 3714,
        layers: &["hg011401_h01", "hg011401_h04"],
    },
    Row {
        flag: 3715,
        layers: &["hg011401_h01", "hg011401_h02"],
    },
    Row {
        flag: 3716,
        layers: &["hg011401_h01", "hg011401_h05"],
    },
    Row {
        flag: 3717,
        layers: &["hg011402_h06", "hg011401_h05"],
    },
    Row {
        flag: 3718,
        layers: &["hg011402_h06"],
    },
    Row {
        flag: 3719,
        layers: &["hg011502_h01"],
    },
    Row {
        flag: 3720,
        layers: &["hg011502_h01", "hg011501_h02"],
    },
    Row {
        flag: 3721,
        layers: &["hg011502_h01", "hg011501_h03"],
    },
    Row {
        flag: 3722,
        layers: &["hg011501_h01", "hg011501_h03"],
    },
    Row {
        flag: 3723,
        layers: &["hg011601_h01"],
    },
    Row {
        flag: 3724,
        layers: &["hg011602_h02"],
    },
    Row {
        flag: 3725,
        layers: &["hg011701_h01"],
    },
    Row {
        flag: 3726,
        layers: &["hg011701_h01", "hg011701_h02", "hg011701_h10"],
    },
    Row {
        flag: 3727,
        layers: &["hg011701_h01", "hg011701_h03", "hg011701_h10"],
    },
    Row {
        flag: 3728,
        layers: &["hg011701_h01", "hg011701_h03", "hg011701_h11"],
    },
    Row {
        flag: 3729,
        layers: &["hg011701_h01", "hg011701_h05", "hg011701_h11"],
    },
    Row {
        flag: 3730,
        layers: &["hg011701_h01", "hg011701_h04", "hg011701_h11"],
    },
    Row {
        flag: 3731,
        layers: &["hg011701_h01", "hg011702_h06", "hg011701_h10"],
    },
    Row {
        flag: 3732,
        layers: &["hg011701_h01", "hg011702_h07", "hg011701_h10"],
    },
    Row {
        flag: 3733,
        layers: &["hg011701_h01", "hg011702_h08", "hg011701_h10"],
    },
    Row {
        flag: 3734,
        layers: &["hg011703_h06", "hg011703_h08"],
    },
    Row {
        flag: 3735,
        layers: &["hg011704_h06"],
    },
    Row {
        flag: 3736,
        layers: &["hg011705_h06"],
    },
    Row {
        flag: 3737,
        layers: &["hg011706_h08"],
    },
    Row {
        flag: 3738,
        layers: &["hg011706_h08", "hg011706_h07"],
    },
    Row {
        flag: 3740,
        layers: &["hg011801_h01"],
    },
    Row {
        flag: 3741,
        layers: &["hg011801_h01", "hg011801_h02"],
    },
    Row {
        flag: 3742,
        layers: &["hg011801_h01", "hg011801_h03"],
    },
    Row {
        flag: 3743,
        layers: &["hg011801_h01", "hg011801_h04"],
    },
    Row {
        flag: 3744,
        layers: &["hg011801_h01", "hg011801_h05"],
    },
    Row {
        flag: 3745,
        layers: &["hg011901"],
    },
    Row {
        flag: 3746,
        layers: &["hg011902"],
    },
    Row {
        flag: 3747,
        layers: &["hg011903"],
    },
    Row {
        flag: 3748,
        layers: &["hg012001_h01"],
    },
    Row {
        flag: 3749,
        layers: &["hg012001_h01", "hg012001_h02"],
    },
    Row {
        flag: 3750,
        layers: &["hg012001_h01", "hg012001_h03"],
    },
    Row {
        flag: 3751,
        layers: &["hg012001_h01", "hg012001_h04"],
    },
    Row {
        flag: 3752,
        layers: &["hg012001_h01", "hg012001_h05"],
    },
    Row {
        flag: 3753,
        layers: &["hg012002_h01", "hg012001_h05"],
    },
    Row {
        flag: 3754,
        layers: &["hg012002_h01"],
    },
    Row {
        flag: 3755,
        layers: &["hg012101_h01"],
    },
    Row {
        flag: 3756,
        layers: &["hg012101_h01", "hg012101_h02"],
    },
    Row {
        flag: 3757,
        layers: &["hg012101_h01", "hg012101_h03"],
    },
    Row {
        flag: 3758,
        layers: &["hg012101_h01", "hg012101_h04"],
    },
    Row {
        flag: 3759,
        layers: &["hg012101_h01", "hg012101_h05"],
    },
    Row {
        flag: 3760,
        layers: &["hg012101_h01", "hg012101_h06"],
    },
    Row {
        flag: 3761,
        layers: &["hg012200"],
    },
    Row {
        flag: 3762,
        layers: &["hg020601_h01"],
    },
    Row {
        flag: 3763,
        layers: &["hg020602_h01", "hg020601_h02"],
    },
    Row {
        flag: 3764,
        layers: &["hg020602_h01"],
    },
    Row {
        flag: 3765,
        layers: &["hg020602_h01", "hg020601_h03"],
    },
    Row {
        flag: 3766,
        layers: &["hg020602_h01", "hg020601_h04"],
    },
    Row {
        flag: 3767,
        layers: &["hg020603_h01", "hg020601_h04"],
    },
    Row {
        flag: 3768,
        layers: &["hg020603_h01", "hg020601_h05"],
    },
    Row {
        flag: 3769,
        layers: &["hg020701_h01"],
    },
    Row {
        flag: 3770,
        layers: &["hg020701_h01", "hg020701_h02"],
    },
    Row {
        flag: 3771,
        layers: &["hg020701_h01", "hg020701_h02", "hg020701_h05"],
    },
    Row {
        flag: 3772,
        layers: &["hg020701_h01", "hg020701_h05"],
    },
    Row {
        flag: 3773,
        layers: &["hg020701_h01", "hg020701_h04"],
    },
    Row {
        flag: 3774,
        layers: &["hg020702_h01", "hg020701_h05"],
    },
    Row {
        flag: 3775,
        layers: &["hg020702_h01", "hg020701_h05", "hg020701_h02"],
    },
    Row {
        flag: 3776,
        layers: &["hg020702_h01", "hg020701_h04"],
    },
    Row {
        flag: 3777,
        layers: &["hg020702_h01", "hg020701_h06"],
    },
    Row {
        flag: 3778,
        layers: &["hg020702_h01", "hg020701_h06", "hg020701_h02"],
    },
    Row {
        flag: 3779,
        layers: &["hg020703_h03", "hg020701_h05"],
    },
    Row {
        flag: 3780,
        layers: &["hg020704_h03", "hg020701_h05"],
    },
    Row {
        flag: 3781,
        layers: &["hg020704_h03"],
    },
    Row {
        flag: 3782,
        layers: &["hg020704_h03", "hg020703_h04"],
    },
    Row {
        flag: 3783,
        layers: &["hg020705_h01"],
    },
    Row {
        flag: 3784,
        layers: &["hg020801_h01"],
    },
    Row {
        flag: 3785,
        layers: &["hg020801_h01", "hg020801_h07"],
    },
    Row {
        flag: 3786,
        layers: &["hg020801_h01", "hg020801_h09"],
    },
    Row {
        flag: 3787,
        layers: &["hg020801_h01", "hg020801_h09", "hg020801_h02"],
    },
    Row {
        flag: 3788,
        layers: &["hg020801_h01", "hg020801_h08", "hg020801_h02"],
    },
    Row {
        flag: 3789,
        layers: &["hg020801_h01", "hg020801_h08", "hg020801_h05"],
    },
    Row {
        flag: 3790,
        layers: &["hg020801_h01", "hg020801_h08", "hg020801_h06"],
    },
    Row {
        flag: 3791,
        layers: &["hg020801_h01", "hg020801_h07", "hg020801_h06"],
    },
    Row {
        flag: 3792,
        layers: &["hg020801_h01", "hg020801_h07", "hg020801_h02"],
    },
    Row {
        flag: 3793,
        layers: &["hg020801_h01", "hg020801_h07", "hg020801_h03"],
    },
    Row {
        flag: 3794,
        layers: &["hg020801_h01", "hg020801_h08", "hg020801_h03"],
    },
    Row {
        flag: 3795,
        layers: &["hg020801_h01", "hg020801_h07", "hg020801_h05"],
    },
    Row {
        flag: 3796,
        layers: &["hg020801_h01", "hg020801_h09", "hg020801_h05"],
    },
    Row {
        flag: 3797,
        layers: &["hg020801_h01", "hg020801_h09", "hg020801_h04"],
    },
    Row {
        flag: 3798,
        layers: &["hg020801_h01", "hg020801_h07", "hg020801_h04"],
    },
    Row {
        flag: 3799,
        layers: &["hg020801_h01", "hg020801_h11", "hg020801_h02"],
    },
    Row {
        flag: 3800,
        layers: &["hg020801_h01", "hg020801_h11", "hg020801_h04"],
    },
    Row {
        flag: 3801,
        layers: &["hg020801_h01", "hg020801_h11", "hg020801_h06"],
    },
    Row {
        flag: 3802,
        layers: &["hg020801_h01", "hg020801_h10", "hg020801_h06"],
    },
    Row {
        flag: 3803,
        layers: &["hg020801_h01", "hg020801_h08", "hg020801_h04"],
    },
    Row {
        flag: 3804,
        layers: &["hg020900"],
    },
    Row {
        flag: 3805,
        layers: &["hg030501_h01"],
    },
    Row {
        flag: 3806,
        layers: &["hg030502_h01", "hg030502_h02"],
    },
    Row {
        flag: 3807,
        layers: &["hg030502_h01", "hg030502_h03"],
    },
    Row {
        flag: 3808,
        layers: &["hg030601_h01"],
    },
    Row {
        flag: 3809,
        layers: &["hg030602_h01", "hg030602_h07"],
    },
    Row {
        flag: 3810,
        layers: &["hg030602_h01", "hg030602_h02"],
    },
    Row {
        flag: 3811,
        layers: &["hg030602_h01", "hg030602_h02", "hg030602_h06"],
    },
    Row {
        flag: 3812,
        layers: &["hg030602_h01", "hg030602_h02", "hg030602_h05"],
    },
    Row {
        flag: 3813,
        layers: &["hg030602_h01", "hg030602_h05"],
    },
    Row {
        flag: 3814,
        layers: &["hg030602_h01", "hg030602_h05", "hg030602_h03"],
    },
    Row {
        flag: 3815,
        layers: &["hg030602_h01", "hg030602_h06", "hg030602_h03"],
    },
    Row {
        flag: 3816,
        layers: &["hg030602_h01", "hg030602_h04"],
    },
    Row {
        flag: 3817,
        layers: &["hg030602_h01", "hg030602_h04", "hg030602_h06"],
    },
    Row {
        flag: 3818,
        layers: &["hg030603"],
    },
    Row {
        flag: 3819,
        layers: &["hg030604"],
    },
    Row {
        flag: 3820,
        layers: &["hg030701_h01"],
    },
    Row {
        flag: 3821,
        layers: &["hg030701_h01", "hg030701_h02"],
    },
    Row {
        flag: 3822,
        layers: &["hg030701_h01", "hg030701_h03"],
    },
    Row {
        flag: 3823,
        layers: &["hg030701_h01", "hg030701_h03", "hg030701_h06"],
    },
    Row {
        flag: 3824,
        layers: &["hg030701_h01", "hg030701_h02", "hg030701_h06"],
    },
    Row {
        flag: 3825,
        layers: &["hg030701_h01", "hg030701_h04"],
    },
    Row {
        flag: 3826,
        layers: &["hg030702_h01", "hg030701_h04"],
    },
    Row {
        flag: 3827,
        layers: &["hg030702_h01", "hg030701_h03"],
    },
    Row {
        flag: 3828,
        layers: &["hg030801_h01"],
    },
    Row {
        flag: 3829,
        layers: &["hg030801_h01", "hg030801_h02"],
    },
    Row {
        flag: 3830,
        layers: &["hg030802_h01", "hg030801_h02"],
    },
    Row {
        flag: 3831,
        layers: &["hg030802_h01", "hg030801_h03"],
    },
    Row {
        flag: 3832,
        layers: &["hg030802_h01", "hg030801_h04"],
    },
    Row {
        flag: 3833,
        layers: &["hg030901_h01"],
    },
    Row {
        flag: 3834,
        layers: &["hg030901_h01", "hg030901_h02"],
    },
    Row {
        flag: 3835,
        layers: &["hg030901_h01", "hg030901_h03"],
    },
    Row {
        flag: 3836,
        layers: &["hg030901_h01", "hg030901_h04"],
    },
    Row {
        flag: 3837,
        layers: &["hg030901_h01", "hg030901_h05"],
    },
    Row {
        flag: 3838,
        layers: &["hg030901_h01", "hg030901_h06"],
    },
    Row {
        flag: 3839,
        layers: &["hg031000"],
    },
    Row {
        flag: 3840,
        layers: &["movieskip"],
    },
    Row {
        flag: 3841,
        layers: &["staff03"],
    },
    Row {
        flag: 3842,
        layers: &["staff01"],
    },
    Row {
        flag: 3843,
        layers: &["staff02"],
    },
    Row {
        flag: 3844,
        layers: &["staff05"],
    },
    Row {
        flag: 3845,
        layers: &["staff04"],
    },
];
