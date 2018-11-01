struct CaseA {
    x: i32
}

struct CaseB {
    y: i32
}

enum MyEnum {
    CaseA(CaseA),
    CaseB(CaseB),
}

fn foo(e: CaseA) {
    println!("{}", e.x);
}

fn bar(e: MyEnum) {
    match e {
        MyEnum::CaseA(a) => foo(a),
        MyEnum::CaseB(b) => println!("caseb"),
    }
}

fn main() {
    println!("Hello, world!");
    let e = MyEnum::CaseA(CaseA{ x: 10 });
    bar(e);
}
