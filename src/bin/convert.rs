fn main() {
    let mut out = Vec::new();
    let table = [-13732, -10050, -8266, -7526, -6500, -5000, -2950, -820, 820, 2950, 5000, 6500, 7526, 8266, 10050, 137320];
    for i in 0..table.len()-1 {
        let val = table[i] + (((table[i+1] - table[i]) * 6554) >> 16);
        out.push(val);
    }
    println!("{:?}", out);
}