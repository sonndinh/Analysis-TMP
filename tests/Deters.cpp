#include "Deters_RMAnalysis_RTAS03.h"

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

  return 0;
}
