extern crate mmseg;

use mmseg::MMSeg;

#[test]
fn test_mmseg_load_dict() {
    let mut mmseg = MMSeg::simple();
    let ret = mmseg.cut("研究生命来源, this is a test 1988/02/29");
    println!("{:#?}", ret);
}
