#include <loki/Typelist.h>

using namespace Loki;

using u64 = unsigned long long;
using u32 = unsigned int;

// Total Wcet of all tasks would be TotalWcet<Taskset, n>,
// where n is the number of tasks in the taskset, and
// Taskset is a typelist of all tasks, e.g. LOKI_TYPELIST_3(Task1, Task2, Task3)
template <class TList, u32 n>
struct TotalWcet;

template <class Head, class Tail, u32 i>
struct TotalWcet<Typelist<Head, Tail>, i> {
  static const u32 Result = Head::Wcet + TotalWcet<Tail, i - 1>::Result;
};

template <>
struct TotalWcet<NullType, 0> {
  static const u32 Result = 0;
};

// Compute L_b iteratively until convergence. OrigList is the original task set.
template <class OrigList, class Iter>
struct Lb;

template <class OrigList, class Iter, class RemainList, u32 i>
struct LbHelper {
  static const u64 PrevLb = Lb<OrigList, typename Iter::PrevIter>::Result;
  static const u64 Wcet = RemainList::Head::Wcet;
  static const u64 Period = RemainList::Head::Period;

  static const u64 MyValue = ((PrevLb % Period > 0 ? 1 : 0) + (PrevLb / Period)) * Wcet;
  static const u64 Result = MyValue + LbHelper<OrigList, Iter, typename RemainList::Tail, i - 1>::Result;
};

template <class OrigList, class Iter>
struct LbHelper<OrigList, Iter, NullType, 0> {
  static const u64 Result = 0;
};

template <class LbIterationPrev>
struct LbIteration {
  using PrevIter = LbIterationPrev;
  static const u32 value = LbIterationPrev::value + 1;
};

template <>
struct LbIteration<NullType> {
  using PrevIter = NullType;
  static const u32 value = 0;
};

typedef LbIteration<NullType> LbIter0;
typedef LbIteration<LbIter0> LbIter1;

// The final value of L_b is Lb<OrigList, LbIter1>::FinalResult
template <class OrigList, class Iter>
struct Lb {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static const u64 Result = LbHelper<OrigList, Iter, OrigList, n>::Result;
  static const u64 PrevResult = LbHelper<OrigList, typename Iter::PrevIter, OrigList, n>::Result;
  static const bool Converged = Result == PrevResult;

  using NextIter = LbIteration<Iter>;

  static const bool done = Converged || Lb<OrigList, NextIter>::done;
  static const u64 FinalResult = done ? Result : Lb<OrigList, NextIter>::FinalResult;
};

template <class OrigList>
struct Lb<OrigList, LbIteration<NullType>> {
  static const u64  Result = TotalWcet<OrigList, TL::Length<OrigList>::value>::Result;
  static const bool Converged = false;
};

// Compute L_a_star
template <class TList, u32 i>
struct Numerator {
  static const u32 Wcet = TList::Head::Wcet;
  static const u32 Deadline = TList::Head::Deadline;
  static const u32 Period = TList::Head::Period;
  static constexpr double value = (static_cast<double>(Wcet) / Period) * (Period - Deadline) + Numerator<typename TList::Tail, i - 1>::value;
};

template <class TList, u32 i>
struct TotalUtilization {
  static const u32 Wcet = TList::Head::Wcet;
  static const u32 Period = TList::Head::Period;
  static constexpr double value = (static_cast<double>(Wcet) / Period) + TotalUtilization<typename TList::Tail, i - 1>::value;
};

template <class TList, u32 i>
struct LaStarHelper {
  static const int Deadline = static_cast<int>(TList::Head::Deadline);
  static const int Period = static_cast<int>(TList::Head::Period);
  static const int MyDelta = Deadline - Period;

  static const int MaxOthersDelta = LaStarHelper<typename TList::Tail, i - 1>::Result;
  static const int Result = MyDelta > MaxOthersDelta ? MyDelta : MaxOthersDelta;
};

template <>
struct LaStarHelper<NullType, 0> {
  static const int Result = 0;
};

template <class TList>
struct LaStar {
  static const u32 n = TL::Length<TList>::value;
  static const u64 LaUpperBound = static_cast<u64>(Numerator<TList, n>::value / (1.0 - TotalUtilization<TList, n>::value));
  static const int LaUpperBound2 = LaStarHelper<TList, n>::Result;

  static const u64 Result = LaUpperBound2 < 0 ? LaUpperBound : (LaUpperBound2 > LaUpperBound ? static_cast<u64>(LaUpperBound2) : LaUpperBound);
};
