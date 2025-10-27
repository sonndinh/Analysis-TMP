#include "Zhang_QPA_ECRTS09.h"

#include <iostream>

int main() {

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

  std::cout << "Task set total WCET: " << TotalWcet<Taskset, 2>::result << std::endl;
  std::cout << "L_b: " << Lb<Taskset>::result << std::endl;
  std::cout << "L_a_star: " << LaStar<Taskset>::result << std::endl;
  std::cout << "L: " << L<Taskset>::result << std::endl;
  {
    std::cout << "Dmax: " << Dmax<Taskset, Taskset, L<Taskset>::result>::result << std::endl;
    std::cout << "DmaxHelper for Task1: " << DmaxHelper<Task1::period, Task1::deadline, L<Taskset>::result>::result << std::endl;
    std::cout << "DmaxHelper for Task2: " << DmaxHelper<Task2::period, Task2::deadline, L<Taskset>::result>::result << std::endl;
  }
  std::cout << "d_min: " << Dmin<Taskset>::result << std::endl;
  std::cout << "h_t for t = " << QPA<Taskset>::t << ": " << QPAHelper<Taskset, QPA<Taskset>::t>::h_t << std::endl;
  std::cout << "keep_going: " << QPAHelper<Taskset, QPA<Taskset>::t>::keep_going << std::endl;
  std::cout << "new_t: " << QPAHelper<Taskset, QPA<Taskset>::t>::new_t << std::endl;
  std::cout << "Task set is schedulable: " << (QPA<Taskset>::schedulable() ? "yes" : "no") << std::endl;

  return 0;
}
