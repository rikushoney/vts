use vts_core::ir;

#[test]
fn test_parse() {
    let sr = ir::from_str(include_str!("sr.llhd")).unwrap();
    sr.verify();

    let add4 = ir::from_str(include_str!("add4.llhd")).unwrap();
    add4.verify();

    let alu4 = ir::from_str(include_str!("alu4.llhd")).unwrap();
    alu4.verify();

    let crc16 = ir::from_str(include_str!("crc16.llhd")).unwrap();
    crc16.verify();

    let crc8 = ir::from_str(include_str!("crc8.llhd")).unwrap();
    crc8.verify();
}
