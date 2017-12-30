
mod instruction {

    use instructions::*;

    #[test]
    fn test_octal_digit_from_binary_list_i16() {

        assert_eq!(octal_digit_from_binary_list_i16(&[1,0,0,1,1,1,0,0]), -100);
        assert_eq!(octal_digit_from_binary_list_i16(&[0,1,1,0,0,1,0,0]), 100);
        assert_eq!(octal_digit_from_binary_list_i16(&[0,0,0,0,0,0,0,1]), 1);
        assert_eq!(octal_digit_from_binary_list_i16(&[1,1,1,1,1,1,1,1]), -1);

    }

    #[test]
    fn test_octal_digit() {
        assert_eq!(octal_digit_from_binary_list(&[0,0,0,1]), 1);
        assert_eq!(octal_digit_from_binary_list(&[1,0,0]), 4);
        assert_eq!(octal_digit_from_binary_list(&[1,1,1,1,1,1,1]), 127);
        assert_eq!(octal_digit_from_binary_list(&[1,1,1,1,1,1,0]), 126);
        assert_eq!(octal_digit_from_binary_list(&[0,1,1,1,1,1,0]), 126-64);

    }

    #[test]
    fn test_octal_digit_u16() {
        assert_eq!(octal_digit_from_binary_list_u16(&[0,0,0,1]), 1);
        assert_eq!(octal_digit_from_binary_list_u16(&[1,0,0]), 4);
        assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1]), 127);
        assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,0]), 126);
        assert_eq!(octal_digit_from_binary_list_u16(&[0,1,1,1,1,1,0]), 126-64);

        assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1]), 255);
        assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1,1]), 511);
        assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1,1,1]), 1023);
        assert_eq!(octal_digit_from_binary_list_u16(&[1,1,1,1,1,1,1,1,1,1,1]), 2047);

    }
}

mod share {

    use share::*;

    #[test]
    fn test_as_i8() {
        let smartbinary0 = SmartBinary::new(0);
        let smartbinary1 = SmartBinary::new(1);
        let smartbinary100 = SmartBinary::new(100);

        assert_eq!(smartbinary100.as_i8(), 100 as i8);
        assert_eq!(smartbinary0.as_i8(), 0 as i8);
        assert_eq!(smartbinary1.as_i8(), 1 as i8);
    }
}
