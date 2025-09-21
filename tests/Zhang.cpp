#include "Zhang_QPA_ECRTS09.h"

#include <iostream>

int main() {

  struct Task1 {
    enum {
      Wcet = 10,
      Deadline = 20,
      Period = 25
    };
  };

  struct Task2 {
    enum {
      Wcet = 15,
      Deadline = 25,
      Period = 30
    };
  };

  using Taskset = LOKI_TYPELIST_2(Task1, Task2);

  std::cout << "Task set total WCET: " << TotalWcet<Taskset, 2>::Result << std::endl;

  return 0;
}
