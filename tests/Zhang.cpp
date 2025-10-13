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
  std::cout << "Task set is schedulable: " << (QPA<Taskset>::schedulable ? "yes" : "no") << std::endl;

  return 0;
}
