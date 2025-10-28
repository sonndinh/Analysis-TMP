#include "Zhang_QPA_ECRTS09.h"

#include <iostream>

template <typename Taskset>
void test_common() {
  const u32 n = static_cast<u32>(TL::Length<Taskset>::value);
  std::cout << "==============" << std::endl;
  std::cout << "Task set total WCET: " << TotalWcet<Taskset, n>::result << std::endl;
  std::cout << "Task set total utilization: " << TotalUtilization<Taskset, n>::value << std::endl;
  std::cout << "L_b: " << Lb<Taskset>::result << std::endl;
  std::cout << "L_a_star: " << LaStar<Taskset>::result << std::endl;
  std::cout << "L: " << L<Taskset>::result << std::endl;
  std::cout << "Dmax: " << Dmax<Taskset, Taskset, L<Taskset>::result>::result() << std::endl;
  std::cout << "d_min: " << Dmin<Taskset>::result << std::endl;
  std::cout << "h_t for t = " << QPA<Taskset>::t << ": " << QPAHelper<Taskset, QPA<Taskset>::t>::h_t << std::endl;
  std::cout << "keep_going: " << QPAHelper<Taskset, QPA<Taskset>::t>::keep_going << std::endl;
  std::cout << "new_t: " << QPAHelper<Taskset, QPA<Taskset>::t>::new_t << std::endl;
  std::cout << "Task set is schedulable: " << (QPA<Taskset>::schedulable() ? "yes" : "no") << '\n' << std::endl;
}

void test1() {
  struct Task1 {
    enum {
      wcet = 10,
      deadline = 20,
      period = 25
    };
  };

  struct Task2 {
    enum {
      wcet = 15,
      deadline = 25,
      period = 30
    };
  };

  using Taskset = LOKI_TYPELIST_2(Task1, Task2);
  test_common<Taskset>();
}

void test2() {
  struct Task1 {
    enum {
      wcet = 5,
      deadline = 15,
      period = 15
    };
  };

  struct Task2 {
    enum {
      wcet = 10,
      deadline = 20,
      period = 30
    };
  };

  struct Task3 {
    enum {
      wcet = 4,
      deadline = 10,
      period = 14
    };
  };
  using Taskset = LOKI_TYPELIST_3(Task1, Task2, Task3);
  test_common<Taskset>();
}

void test3() {
  struct Task1 {
    enum {
      wcet = 6000,
      deadline = 18000,
      period = 31000
    };
  };

  struct Task2 {
    enum {
      wcet = 2000,
      deadline = 9000,
      period = 9800
    };
  };

  struct Task3 {
    enum {
      wcet = 1000,
      deadline = 12000,
      period = 17000
    };
  };

  struct Task4 {
    enum {
      wcet = 90,
      deadline = 3000,
      period = 4200
    };
  };

  struct Task5 {
    enum {
      wcet = 8,
      deadline = 10,
      period = 96
    };
  };

  struct Task6 {
    enum {
      wcet = 2,
      deadline = 16,
      period = 12
    };
  };

  struct Task7 {
    enum {
      wcet = 10,
      deadline = 19,
      period = 280
    };
  };

  struct Task8 {
    enum {
      wcet = 26,
      deadline = 160,
      period = 660
    };
  };

  using Taskset = LOKI_TYPELIST_8(Task1, Task2, Task3, Task4, Task5, Task6, Task7, Task8);
  test_common<Taskset>();
}

int main() {
  test1();
  test2();
  test3();
  return 0;
}
