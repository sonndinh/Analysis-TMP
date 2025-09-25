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

// Compute L_b iteratively until convergence. OrigList is the original task set.
template <class OrigList, class Iter>
struct Lb;

template <class OrigList, class Iter, class RemainList, unsigned int i>
struct LbHelper {
  enum {
    PrevLb = Lb<OrigList, typename Iter::PrevIter>::Result,

    Wcet = RemainList::Head::Wcet,
    Period = RemainList::Head::Period,

    MyValue = ((PrevLb % Period > 0 ? 1 : 0) + (PrevLb / Period)) * Wcet,

    Result = MyValue + LbHelper<OrigList, Iter, typename RemainList::Tail, i - 1>::Result
  };
};

template <class OrigList, class Iter>
struct LbHelper<OrigList, Iter, NullType, 0> {
  enum { Result = 0 };
};

template <class LbIterationPrev>
struct LbIteration {
  typedef LbIterationPrev PrevIter;

  static const unsigned int value = LbIterationPrev::value + 1;
};

template <>
struct LbIteration<NullType> {
  typedef NullType PrevIter;

  static const unsigned int value = 0;
};

typedef LbIteration<NullType> LbIter0;
typedef LbIteration<LbIter0> LbIter1;

// The final value of L_b is Lb<OrigList, LbIter1>::FinalResult
template <class OrigList, class Iter>
struct Lb {
  enum {
    n = TL::Length<OrigList>::value,

    Result = LbHelper<OrigList, Iter, OrigList, n>::Result,

    PrevResult = LbHelper<OrigList, typename Iter::PrevIter, OrigList, n>::Result,

    Converged = Result == PrevResult
  };

  typedef LbIteration<Iter> NextIter;

  static const bool done = Converged || Lb<OrigList, NextIter>::done;
  static const unsigned int FinalResult = done ? Result : Lb<OrigList, NextIter>::FinalResult;
};

template <class OrigList>
struct Lb<OrigList, LbIteration<NullType>> {
  enum {
    Result = TotalWcet<OrigList, TL::Length<OrigList>::value>::Result,
    Converged = false
  };
};

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
