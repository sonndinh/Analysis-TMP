use super::*;
use typenum::{Bit, Integer};
use typenum::{P2, P3, P4, P5, P6, P7, P8, P9, P10, P12, P14, P15, P16, P17, P18, P19, P26, P31, P90, P96, P100, P160, P200, P280, P404, P660, P800, P1000, Z0};

type P2000 = typenum::op!(P1000 * P2);
type P3000 = typenum::op!(P1000 * P3);
type P4200 = typenum::op!(P1000 * P4 + P200);
type P6000 = typenum::op!(P1000 * P6);
type P9000 = typenum::op!(P1000 * P9);
type P9800 = typenum::op!(P1000 * P9 + P800);
type P12000 = typenum::op!(P1000 * P12);
type P15404 = typenum::op!(P1000 * P15 + P404);
type P17000 = typenum::op!(P1000 * P17);
type P18000 = typenum::op!(P1000 * P18);
type P31000 = typenum::op!(P1000 * P31);

#[test]
fn test1() {
    type PdfOf100 = <Taskset as Pdf<P100>>::Output;
    println!("h(100): {}", <PdfOf100 as Integer>::to_i32());

    type DmaxOf100 = <Taskset as Dmax<P100>>::Output;
    println!("Dmax(100): {}", <DmaxOf100 as Integer>::to_i32());

    type QpaConditionOf100 = <Taskset as QpaCondition<P100>>::Output;
    println!("QpaCondition(100): {}", <QpaConditionOf100 as Bit>::to_bool());

    type QpaConditionOf200 = <Taskset as QpaCondition<P200>>::Output;
    println!("QpaCondition(200): {}", <QpaConditionOf200 as Bit>::to_bool());

    type QpaOf100 = <(Task1, Tasklist<Task2, Nulltask>, P100) as Qpa>::Output;
    println!("Qpa(100): {}", <QpaOf100 as Bit>::to_bool());

    type QpaOf12 = <(Task1, Tasklist<Task2, Nulltask>, P12) as Qpa>::Output;
    println!("Qpa(12): {}", <QpaOf12 as Bit>::to_bool());

    struct Task1;
    impl Task for Task1 {
        type Wcet = P5;
        type Deadline = P10;
        type Period = P15;
    }

    struct Task2;
    impl Task for Task2 {
        type Wcet = P7;
        type Deadline = P12;
        type Period = P14;
    }

    type Taskset = Tasklist<Task1, Tasklist<Task2, Nulltask>>;

    type Y = <<Task1 as Task>::Wcet as Div<<Task1 as Task>::Period>>::Output;
    let util = <Y as Integer>::I32;
    println!("Utilization of task1: {}", util);

    type SumWcet = <Taskset as TotalWcet>::Output;
    let total_wcet = <SumWcet as Integer>::to_i32();
    println!("Total wcet: {}", total_wcet);

    type MinDeadline = <Taskset as Dmin>::Output;
    println!("Dmin: {}", <MinDeadline as Integer>::to_i32());

    type MyLb = <(Task1, Tasklist<Task2, Nulltask>, Z0, SumWcet) as Lb>::Output;
    assert_eq!(<MyLb as Integer>::to_i32(), 12);
    println!("Lb: {}", <MyLb as Integer>::to_i32());

    type DmaxOf12 = <Taskset as Dmax<P12>>::Output;
    assert_eq!(<DmaxOf12 as Integer>::to_i32(), 10);
    println!("Dmax(Lb=12): {}", <DmaxOf12 as Integer>::to_i32());

    type PdfOf10 = <Taskset as Pdf<P10>>::Output;
    assert_eq!(<PdfOf10 as Integer>::to_i32(), 5);
    println!("h(t=10): {}", <PdfOf10 as Integer>::to_i32());

    type MyQpa = <(Task1, Tasklist<Task2, Nulltask>, MyLb) as Qpa>::Output;
    assert!(<MyQpa as Bit>::to_bool());
    println!("Qpa: {}", <MyQpa as Bit>::to_bool());
}

#[test]
fn test2() {
    struct Task1;
    impl Task for Task1 {
        type Wcet = P6000;
        type Deadline = P18000;
        type Period = P31000;
    }

    struct Task2;
    impl Task for Task2 {
        type Wcet = P2000;
        type Deadline = P9000;
        type Period = P9800;
    }

    struct Task3;
    impl Task for Task3 {
        type Wcet = P1000;
        type Deadline = P12000;
        type Period = P17000;
    }

    struct Task4;
    impl Task for Task4 {
        type Wcet = P90;
        type Deadline = P3000;
        type Period = P4200;
    }

    struct Task5;
    impl Task for Task5 {
        type Wcet = P8;
        type Deadline = P10;
        type Period = P96;
    }

    struct Task6;
    impl Task for Task6 {
        type Wcet = P2;
        type Deadline = P16;
        type Period = P12;
    }

    struct Task7;
    impl Task for Task7 {
        type Wcet = P10;
        type Deadline = P19;
        type Period = P280;
    }

    struct Task8;
    impl Task for Task8 {
        type Wcet = P26;
        type Deadline = P160;
        type Period = P660;
    }

    type RemainingTasks = Tasklist<Task2, Tasklist<Task3, Tasklist<Task4, Tasklist<Task5, Tasklist<Task6, Tasklist<Task7, Tasklist<Task8, Nulltask>>>>>>>;
    type Taskset = Tasklist<Task1, RemainingTasks>;

    type SumWcet = <Taskset as TotalWcet>::Output;
    let total_wcet = <SumWcet as Integer>::to_i32();
    println!("Total wcet: {}", total_wcet);

    type MyLb = <(Task1, RemainingTasks, Z0, SumWcet) as Lb>::Output;
    assert_eq!(<MyLb as Integer>::to_i32(), 16984);
    println!("Lb: {}", <MyLb as Integer>::to_i32());

    type Dmax16984 = <Tasklist<Task1, RemainingTasks> as Dmax<MyLb>>::Output;
    println!("Dmax(16984): {}", <Dmax16984 as Integer>::to_i32());

    // La* is 15404
    type Dmax15404 = <Tasklist<Task1, RemainingTasks> as Dmax<P15404>>::Output;
    assert_eq!(<Dmax15404 as Integer>::to_i32(), 15400);
    println!("Dmax(15404): {}", <Dmax15404 as Integer>::to_i32());

    // Test QPA using La*
    type QpaUsingLaStar = <(Task1, RemainingTasks, Dmax15404) as Qpa>::Output;
    println!("Qpa(La*): {}", <QpaUsingLaStar as Bit>::to_bool());

    // Test QPA using Lb -- should get the same result as when using La*
    type QpaUsingLb = <(Task1, RemainingTasks, Dmax16984) as Qpa>::Output;
    println!("Qpa(Lb): {}", <QpaUsingLb as Bit>::to_bool());

    // Tracing the QPA iterations
    // t = Dmax15404 = 15400
    // type QpaCondition15400 = <Tasklist<Task1, RemainingTasks> as QpaCondition<Dmax15404>>::Output;
    // println!("QpaCondition<15400>: {}", <QpaCondition15400 as Bit>::to_bool());

    type Pdf15400 = PdfValue<Task1, RemainingTasks, Dmax15404>;
    println!("Pdf<15400>: {}", <Pdf15400 as Integer>::to_i32());
}
