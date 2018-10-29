
fn main() {
    let pdf = [6, 5, 11, 31, 132, 21, 8, 4, 3, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    let mut total = 256;
    let mut out = Vec::with_capacity(pdf.len());
    for val in pdf.iter() {
        total -= val;
        out.push(total);
    }
    println!("{:?}", out);
}