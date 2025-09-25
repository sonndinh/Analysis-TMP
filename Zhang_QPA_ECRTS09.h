#include <loki/Typelist.h>

#include <climits>

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
  static const u32 Count = LbIterationPrev::Count + 1;
};

template <>
struct LbIteration<NullType> {
  using PrevIter = NullType;
  static const u32 Count = 0;
};

typedef LbIteration<NullType> LbIter0;
typedef LbIteration<LbIter0> LbIter1;

// The final value of L_b is Lb<OrigList, LbIter1>::FinalResult
template <class OrigList, class Iter>
struct Lb {
  static const u32 N = static_cast<u32>(TL::Length<OrigList>::value);
  static const u64 Result = LbHelper<OrigList, Iter, OrigList, N>::Result;
  static const u64 PrevResult = LbHelper<OrigList, typename Iter::PrevIter, OrigList, N>::Result;
  static const bool Converged = Result == PrevResult;

  using NextIter = LbIteration<Iter>;

  static const bool Done = Converged || Lb<OrigList, NextIter>::Done;
  static const u64 FinalResult = Done ? Result : Lb<OrigList, NextIter>::FinalResult;
};

template <class OrigList>
struct Lb<OrigList, LbIteration<NullType>> {
  static const u64 Result = TotalWcet<OrigList, TL::Length<OrigList>::value>::Result;
  static const bool Converged = false;
};

// Compute L_a_star
template <class TList, u32 i>
struct Numerator {
  static const u32 Wcet = TList::Head::Wcet;
  static const u32 Deadline = TList::Head::Deadline;
  static const u32 Period = TList::Head::Period;
  static constexpr double Value = (static_cast<double>(Wcet) / Period) * (Period - Deadline) + Numerator<typename TList::Tail, i - 1>::Value;
};

template <class TList, u32 i>
struct TotalUtilization {
  static const u32 Wcet = TList::Head::Wcet;
  static const u32 Period = TList::Head::Period;
  static constexpr double Value = (static_cast<double>(Wcet) / Period) + TotalUtilization<typename TList::Tail, i - 1>::Value;
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
  static const u32 N = TL::Length<TList>::value;
  static constexpr double TotalUtil = TotalUtilization<TList, N>::Value;
  static const u64 LaUpperBound = TotalUtil < 1.0 ? static_cast<u64>(Numerator<TList, N>::Value / (1.0 - TotalUtilization<TList, N>::Value)) : ULLONG_MAX;
  static const int LaUpperBound2 = LaStarHelper<TList, N>::Result;

  static const u64 Result = LaUpperBound2 < 0 ? LaUpperBound : (LaUpperBound2 > LaUpperBound ? static_cast<u64>(LaUpperBound2) : LaUpperBound);
};

template <class TList>
struct L {
  static const u64 Result = LaStar<TList>::Result < Lb<TList, LbIter1>::FinalResult ? LaStar<TList>::Result : Lb<TList, LbIter1>::FinalResult;
};
