use crate::models::cli::Mod;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Input {
    pub git_token: String,
    pub nexus_key: String,
    pub gist_id: String,
    pub owner: String,
    pub repo: String,
    pub mods: Vec<Mod>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct ModDetails {
    pub name: String,
    #[serde(skip_deserializing)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing)]
    pub uid: u64,
    #[serde(serialize_with = "serialize_download_ct")]
    pub mod_downloads: usize,
    #[serde(serialize_with = "serialize_download_ct")]
    pub mod_unique_downloads: usize,
}

fn serialize_download_ct<S>(value: &usize, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted = format_download_ct(*value);
    serializer.serialize_str(&formatted)
}

fn format_download_ct(count: usize) -> String {
    if count < 10_000 {
        return count.to_string();
    } else if count >= 1_000_000_000_000 {
        return format!("{count:.1e}");
    }

    let (delta, suf) = {
        const CT_SUF: [char; 3] = ['k', 'M', 'T'];
        let mut delta = count as f64;
        let mut i = 0;

        while delta >= 1000.0 {
            delta /= 1000.0;
            i += 1;
        }

        (delta, CT_SUF[i - 1])
    };

    let precision = {
        let mut precision = 2 - delta.log10().floor() as i32;

        let get_place_dg = |place: i32| -> u8 {
            let shifted = delta * 10_f64.powi(place);
            (shifted.abs() as u64 % 10) as u8
        };

        while precision > 0 && get_place_dg(precision) == 0 {
            precision -= 1
        }

        precision
    };

    if precision == 0 {
        return format!("{}{suf}", delta.trunc());
    }

    let multiplier = 10_f64.powi(precision);
    let truncated = (delta * multiplier).trunc() / multiplier;
    format!("{truncated:.*}{suf}", precision as usize)
}

#[derive(Deserialize)]
pub(crate) struct GistResponse {
    pub id: String,
    pub files: HashMap<String, FileDetails>,
}

#[derive(Deserialize)]
pub(crate) struct FileDetails {
    pub raw_url: String,
    pub content: String,
}

#[derive(Deserialize)]
pub(crate) struct Version {
    pub latest: String,
    pub message: String,
}

#[derive(Deserialize)]
pub(crate) struct RepositoryPublicKey {
    pub key_id: String,
    pub key: String,
}

#[cfg(test)]
mod test {
    use super::format_download_ct;

    macro_rules! test {
        ($input:literal, $output:literal) => {
            assert_eq!(format_download_ct($input), $output)
        };
    }

    #[test]
    fn does_count_format() {
        test!(10_050, "10k");
        test!(10_110, "10.1k");
        test!(406_356, "406k");
        test!(549_999, "549k");
        test!(999_950, "999k");
        test!(1_000_000, "1M");
        test!(2_200_000, "2.2M");
        test!(6_156_000, "6.15M");
        test!(45_425_000, "45.4M");
        test!(346_425_000, "346M");
        test!(3_634_425_000, "3.63T");
        test!(999_999_999_999, "999T");
        test!(5_835_742_000_000, "5.8e12");
        test!(106_634_154_000_000, "1.1e14");
    }
}
