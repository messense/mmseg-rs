extern crate mmseg;

use mmseg::MMSeg;

#[test]
fn test_mmseg() {
    let mmseg = MMSeg::new();
    let simple = mmseg.cut_simple("研究生命来源, this is a test 1988/02/29");
    println!("simple: {:#?}", simple);
    let complex = mmseg
        .cut("我是拖拉机学院手扶拖拉机专业的。不用多久，我就会升职加薪，当上CEO，走上人生巅峰。");
    println!("complex: {:#?}", complex);
}
