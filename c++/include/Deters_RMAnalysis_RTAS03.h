#include <loki/Typelist.h>

using namespace Loki;

template <class TL, int i, int t, int j = 0>
struct sum_j
{
  typedef typename Loki::TL::TypeAt<TL, j>::Result J;

  enum
  {
    Cj = J::cost,

    Tj = J::period,

    my_result = Cj * ((t % Tj > 0 ? 1 : 0) + (t / Tj)),

    Result = sum_j<TL, i, t, j + 1>::Result + my_result
  };
};

template <class TL, int i, int t>
struct sum_j<TL, i, t, i>
{
  enum
  {
    Result = 0
  };
};

template <class TL, int i, int t_ix, int k = 0>
struct get_t
{
  enum
  {
    Ti = Loki::TL::TypeAt<TL, i - 1>::Result::period,

    Tk = Loki::TL::TypeAt<TL, k>::Result::period,

    num_l = Ti / Tk,

    Result = (t_ix >= num_l) ?
      get_t<TL, i, t_ix - num_l, k + 1>::Result
      : (t_ix + 1) * Tk
  };
};

template <class TL, int i, int t_ix>
struct get_t<TL, i, t_ix, i>
{
  enum
  {
    Result = 0
  };
};

template <class TL, int i, int t_ix = 0>
struct task_feasible
{
  typedef get_t<TL, i, t_ix> t_type;
  enum
  {
    t = t_type::Result,

    Result = (t > 0) &&
             (sum_j<TL, i, t>::Result <= t || task_feasible<TL, i, t_ix + 1>::Result)
  };
};

template <class TL, int i>
struct task_feasible<TL, i, i>
{
  enum
  {
    Result = 0
  };
};

template <class TL, int m, int i>
struct check_i;

// Recursive case for check_i
template <class Head, class Tail, int m, int i>
struct check_i<Typelist<Head, Tail>, m, i>
{
  enum
  {
    task_result = task_feasible<Typelist<Head, Tail>, i>::Result,

    Result = check_i<Typelist<Head, Tail>, m, i + 1>::Result && task_result
  };
};

// Base case for check_i
template <class Head, class Tail, int m>
struct check_i<Typelist<Head, Tail>, m, m>
{
  enum
  {
    Result = task_feasible<Typelist<Head, Tail>, m>::Result
  };
};


template <class TaskSet>
struct RMA_Feasible
{
  enum
  {
    m = TL::Length<TaskSet>::value,

    Result = check_i<TaskSet, m, 1>::Result
  };
};

template <class TaskSet>
struct Schedule;

template <class Head, class Tail>
struct Schedule<Typelist<Head, Tail>>
{
  typedef Typelist<Head, Tail> TL;

  enum
  {
    feasible = RMA_Feasible<TL>::Result
  };

  constexpr static const double utilization = Schedule<Tail>::utilization + double(Head::cost) / Head::period;

  static void schedule(void)
  {
    /* (not shown) */
  }
};

template <>
struct Schedule<NullType>
{
  static const bool Result = true;
  constexpr static const double utilization = 0.0;

  static void schedule(void)
  {
    // no action necessary
  }
};
