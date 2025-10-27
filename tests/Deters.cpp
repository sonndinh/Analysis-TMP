#include "Deters_RMAnalysis_RTAS03.h"

#include <iostream>

// TODO: Try similar task sets for both Zhang's and Deters's analyses
void test1() {
  struct Task1 {
    enum {
      cost = 10,
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
}

int main() {
  struct TaskA {
    enum {
      cost = 5,
      period = 10
    };
  };

  struct TaskB {
    enum {
      cost = 5,
      period = 15
    };
  };

  typedef LOKI_TYPELIST_2(TaskA, TaskB) MyTasks;
  const int feasible = RMA_Feasible<MyTasks>::Result;

  std::cout << "The task set is " << (feasible ? "feasible" : "NOT feasible") << std::endl;

  return 0;
}
