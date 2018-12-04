use rstd::prelude::Vec;

pub fn vec_to_u32(date: Vec<u8>) -> Option<u32> {
    let mut frozen_duration  = 0;
    let maxvalue = u32::max_value();
    /// ascii '0' = 48  '9' = 57
    for i in date {
        if i > 57 || i< 48 {
            return None;
        }
        frozen_duration = frozen_duration * 10 + u32::from(i -48);
        if frozen_duration > maxvalue{
            return None;
        }
    }
    Some(frozen_duration)
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_vec_to_u32(){
        let mut date: Vec<u8> = Vec::new();
        date.push(54);
        date.push(54);
        date.push(57);
        date.push(57);
        date.push(48);

        let frozen_duration = if let Some(date) = vec_to_u32(date){
            date
        } else {
            0
        };
        assert_eq!(66990, frozen_duration);
    }
}
