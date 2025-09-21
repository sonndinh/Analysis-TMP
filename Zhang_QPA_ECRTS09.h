#include <loki/Typelist.h>

using namespace Loki;

// Total Wcet of all tasks would be TotalWcet<Taskset, n>,
// where n is the number of tasks in the taskset, and
// Taskset is a typelist of all tasks, e.g. LOKI_TYPELIST_3(Task1, Task2, Task3)
template <class TList, unsigned int n>
struct TotalWcet;

template <class Head, class Tail, unsigned int i>
struct TotalWcet<Typelist<Head, Tail>, i> {
  enum {
    Result = Head::Wcet + TotalWcet<Tail, i - 1>::Result
  };
};

template <>
struct TotalWcet<NullType, 0> {
  enum { Result = 0 };
};

// Compute L_b
template <class TList, unsigned int iter>
struct Lb;

template <class TList, unsigned int iter, unsigned int i>
struct LbHelper {
  enum {
    SumWcet = TotalWcet<TList, TL::Length<TList>::value>::Result,

    PrevLb = Lb<TList, iter - 1>::Result,

    Wcet = TList::Head::Wcet,
    Period = TList::Head::Period,

    MyValue = ((PrevLb % Period > 0 ? 1 : 0) + (PrevLb / Period)) * Wcet,

    Result = MyValue + LbHelper<TList, iter, i - 1>::Result
  };
};

template <class TList, unsigned int iter>
struct LbHelper<TList, iter, 0> {
  enum { Result = 0 };
};

// TODO: stop the recursion when Lb does not change anymore
template <class TList, unsigned int iter>
struct Lb {
  enum {
    n = TL::Length<TList>::value,

    Result = LbHelper<TList, iter, n>::Result
  };
};

template <class TList>
struct Lb<TList, 0> {
  enum {
    Result = TotalWcet<TList, TL::Length<TList>::value>::Result
  };
};

// TODO: static const values can be replaced with constexpr?

// Compute L_a_star
template <class TList, unsigned int i>
struct Numerator {
  static const int Wcet = TList::Head::Wcet;
  static const int Deadline = TList::Head::Deadline;
  static const int Period = TList::Head::Period;
  constexpr static const double value = (static_cast<double>(Wcet) / Period) * (Period - Deadline) + Numerator<typename TList::Tail, i - 1>::value;
};

template <class TList, unsigned int i>
struct TotalUtilization {
  static const int Wcet = TList::Head::Wcet;
  static const int Period = TList::Head::Period;
  constexpr static const double value = (static_cast<double>(Wcet) / Period) + TotalUtilization<typename TList::Tail, i - 1>::value;
};

template <class TList, unsigned int i>
struct LaStarHelper {
  static const int Deadline = TList::Head::Deadline;
  static const int Period = TList::Head::Period;
  static const int tmp = Deadline - Period;;

  static const int Result = tmp > LaStarHelper<typename TList::Tail, i - 1>::Result ? tmp : LaStarHelper<typename TList::Tail, i - 1>::Result;
};

template <>
struct LaStarHelper<NullType, 0> {
  static const int Result = 0;
};

template <class TList>
struct LaStar {
  static const int n = TL::Length<TList>::value;
  static const int LaUpperBound = static_cast<int>(Numerator<TList, n>::value / (1 - TotalUtilization<TList, n>::value));
  static const int LaUpperBound2 = LaStarHelper<TList, n>::Result;
  static const int Result = LaUpperBound2 > LaUpperBound ? LaUpperBound2 : LaUpperBound;
};
