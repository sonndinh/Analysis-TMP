#include <loki/Typelist.h>

#include <climits>

using namespace Loki;

using u64 = unsigned long long;
using i64 = long long;
using u32 = unsigned int;
using i32 = int;

// Total Wcet of all tasks would be TotalWcet<Taskset, n>,
// where n is the number of tasks in the taskset, and
// Taskset is a typelist of all tasks, e.g. LOKI_TYPELIST_3(Task1, Task2, Task3)
template <class TList, u32 n>
struct TotalWcet;

template <class Head, class Tail, u32 i>
struct TotalWcet<Typelist<Head, Tail>, i> {
  static const u32 result = Head::wcet + TotalWcet<Tail, i - 1>::result;
};

template <>
struct TotalWcet<NullType, 0> {
  static const u32 result = 0;
};

// Compute L_b iteratively until convergence. OrigList is the original task set.
template <class OrigList, class Iter>
struct Lb;

template <class OrigList, class Iter, class RemainList, u32 i>
struct LbHelper {
  static const u64 prev_lb = Lb<OrigList, typename Iter::PrevIter>::result;
  static const u64 wcet = RemainList::Head::wcet;
  static const u64 period = RemainList::Head::period;

  static const u64 my_value = ((prev_lb % period > 0 ? 1 : 0) + (prev_lb / period)) * wcet;
  static const u64 result = my_value + LbHelper<OrigList, Iter, typename RemainList::Tail, i - 1>::result;
};

template <class OrigList, class Iter>
struct LbHelper<OrigList, Iter, NullType, 0> {
  static const u64 result = 0;
};

template <class LbIterationPrev>
struct LbIteration {
  using PrevIter = LbIterationPrev;
  static const u32 count = LbIterationPrev::count + 1;
};

template <>
struct LbIteration<NullType> {
  using PrevIter = NullType;
  static const u32 count = 0;
};

typedef LbIteration<NullType> LbIter0;
typedef LbIteration<LbIter0> LbIter1;

// The final value of L_b is Lb<OrigList, LbIter1>::FinalResult
template <class OrigList, class Iter>
struct Lb {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static const u64 result = LbHelper<OrigList, Iter, OrigList, n>::result;
  static const u64 prev_result = LbHelper<OrigList, typename Iter::PrevIter, OrigList, n>::result;
  static const bool converged = result == prev_result;

  using NextIter = LbIteration<Iter>;

  static const bool done = converged || Lb<OrigList, NextIter>::done;
  static const u64 final_result = done ? result : Lb<OrigList, NextIter>::final_result;
};

template <class OrigList>
struct Lb<OrigList, LbIteration<NullType>> {
  static const u64 result = TotalWcet<OrigList, TL::Length<OrigList>::value>::result;
  static const bool converged = false;
};

// Compute L_a_star
template <class TList, u32 i>
struct Numerator {
  static const u32 wcet = TList::Head::wcet;
  static const u32 deadline = TList::Head::deadline;
  static const u32 period = TList::Head::period;
  static constexpr double value = (static_cast<double>(wcet) / period) * (period - deadline) + Numerator<typename TList::Tail, i - 1>::value;
};

template <class TList, u32 i>
struct TotalUtilization {
  static const u32 wcet = TList::Head::wcet;
  static const u32 period = TList::Head::period;
  static constexpr double value = (static_cast<double>(wcet) / period) + TotalUtilization<typename TList::Tail, i - 1>::value;
};

template <class TList, u32 i>
struct LaStarHelper {
  static const int deadline = static_cast<int>(TList::Head::deadline);
  static const int period = static_cast<int>(TList::Head::period);
  static const int my_delta = deadline - period;

  static const int max_others_delta = LaStarHelper<typename TList::Tail, i - 1>::result;
  static const int result = my_delta > max_others_delta ? my_delta : max_others_delta;
};

template <>
struct LaStarHelper<NullType, 0> {
  static const int result = 0;
};

template <class TList>
struct LaStar {
  static const u32 n = TL::Length<TList>::value;
  static constexpr double total_util = TotalUtilization<TList, n>::value;
  static const u64 la_upperbound = total_util < 1.0 ? static_cast<u64>(Numerator<TList, n>::value / (1.0 - TotalUtilization<TList, n>::value)) : ULLONG_MAX;
  static const int la_upperbound_2 = LaStarHelper<TList, n>::result;

  static const u64 result = la_upperbound_2 < 0 ? la_upperbound : (la_upperbound_2 > la_upperbound ? static_cast<u64>(la_upperbound_2) : la_upperbound);
};

template <typename TList>
struct L {
  static const u64 result = LaStar<TList>::result < Lb<TList, LbIter1>::final_result ? LaStar<TList>::result : Lb<TList, LbIter1>::final_result;
};

// Compute Dmin
template <typename TList>
struct Dmin {
  static const u32 result = TList::Head::deadline < Dmin<typename TList::Tail>::result ? TList::Head::deadline : Dmin<typename TList::Tail>::result;
};

template <>
struct Dmin<NullType> {
  static const u32 result = UINT_MAX;
};

// Compute the processor demand function h(t)
template <typename TList, u64 t, u32 i>
struct Pdf;

template <typename TList, u64 t, u32 i>
struct Pdf {
  static const u32 wcet = TList::Head::wcet;
  static const u32 period = TList::Head::period;
  static const u32 deadline = TList::Head::deadline;

  // For negative value, division operator rounds up while the floor operator rounds down.
  static const i64 sub = static_cast<i64>(t - deadline);
  static const i64 floor_value = sub >= 0 ? sub / period : (sub % period == 0 ? sub / period : (sub / period) - 1);
  static const u64 my_value = static_cast<u64>((1 + floor_value) < 0 ? 0 : (1 + floor_value)) * wcet;
  static const u64 result = my_value + Pdf<typename TList::Tail, t, i - 1>::result;
};

template <u64 t>
struct Pdf<NullType, t, 0> {
  static const u64 result = 0;
};

// Maximum absolute deadline of a task that is less than L.
// The following demonstrates cases when deadline is greater than period.
// But this should work for constrained, implicit, and arbitrary deadlines.
// |<----------------------------- length ---------------------------------->|
// |-------------------------------------------------------------------------|
//                        (last release whose deadline < length)  (deadline) |
//                                               |                    |      |
//                                               |<------- D_i ------>|      |
//                                               v                    v      |
// |.............................................|--------------------|------|
//                                                    (next release)      (misses deadline)
//                                                           |               |    |
//                                                           v               |    v
//                                               |<-- T_i -->|<------ D_i ------->|
// |.............................................|-----------|--------------------|
// |.........................................................|<-- remain --->|
template <u32 period, u32 deadline, u64 length>
struct DmaxHelper {
  // Check if deadline < length < period. In this case, we also stop recurring to the next instantiation of DmaxHelper.
  // The last absolute deadline within the given length in this case is different than when length <= deadline since
  // for this case, we don't have to go back to the immediate previous job release.
  static const bool edge_case = length <= deadline ? false : (length < period ? true : DmaxHelper<period, deadline, length - period>::edge_case);
  static const u64 remain = length <= deadline ? length
                                               : (length < period ? length : DmaxHelper<period, deadline, length - period>::remain);
  static const u64 result = edge_case ? (length - remain + deadline) : (length - remain - period + deadline);
};

// Maximum absolute deadline of all tasks in a task set that is less than a given span of lenth len.
// For length L, top-level calls this with Dmax<Taskset, Taskset, L<OrigList>::result>.
template <typename OrigList, typename RemainList, u64 len>
struct Dmax {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static const u32 period = RemainList::Head::period;
  static const u32 deadline = RemainList::Head::deadline;
  static const u64 my_result = DmaxHelper<period, deadline, len>::result;
  static const u64 others_result = Dmax<OrigList, typename RemainList::Tail, len>::result;
  static const u64 result = my_result > others_result ? my_result : others_result;
};

// QPA test
template <typename  OrigList, u64 t>
struct QPAHelper {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static const u64 h_t = Pdf<OrigList, t, n>::result;
  static const u32 d_min = Dmin<OrigList>::result;

  // Update the value of t and recur
  static const bool keep_going = h_t <= t && h_t > d_min;
  static const u64 new_t = keep_going ? (h_t < t ? h_t : Dmax<OrigList, OrigList, t>::result) : 0;
  static const bool result = !keep_going ? (h_t <= d_min ? true : false) : QPAHelper<OrigList, new_t>::result;
};

template <typename OrigList>
struct QPA {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static constexpr double total_util = TotalUtilization<OrigList, n>::value;
  static const u64 t = Dmax<OrigList, OrigList, L<OrigList>::result>::result;
  static const bool schedulable = total_util > 1.0 ? false : QPAHelper<OrigList, t>::result;
};
