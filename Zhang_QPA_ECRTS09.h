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
template <typename TList, u32 n>
struct TotalWcet;

template <typename Head, typename Tail, u32 i>
struct TotalWcet<Typelist<Head, Tail>, i> {
  static const u32 result = Head::wcet + TotalWcet<Tail, i - 1>::result;
};

template <>
struct TotalWcet<NullType, 0> {
  static const u32 result = 0;
};

// Compute L_b iteratively until convergence. OrigList is the original task set.
template <typename OrigList, u64 prev_value, u32 iter, typename RemainList, u32 i>
struct LbHelper2 {
  static const u64 wcet = RemainList::Head::wcet;
  static const u64 period = RemainList::Head::period;

  static const u64 my_value = ((prev_value % period > 0 ? 1 : 0) + (prev_value / period)) * wcet;
  static const u64 result = my_value + LbHelper2<OrigList, prev_value, iter, typename RemainList::Tail, i - 1>::result;
};

template <typename OrigList, u64 prev_value, u32 iter>
struct LbHelper2<OrigList, prev_value, iter, NullType, 0> {
  static const u64 result = 0;
};

template <typename OrigList, u64 prev_value, u32 iter>
struct LbHelper {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static const u64 value = LbHelper2<OrigList, prev_value, iter, OrigList, n>::result;
  static constexpr bool converged = value == prev_value;

  static constexpr u64 result() {
    if constexpr (converged) {
      return value;
    } else {
      return LbHelper<OrigList, value, iter + 1>::result();
    }
  }
};

template <typename OrigList, u64 prev_value>
struct LbHelper<OrigList, prev_value, 0> {
  static constexpr u64 result() {
    return TotalWcet<OrigList, TL::Length<OrigList>::value>::result;
  }
};

template <typename OrigList>
struct Lb {
  static const u64 init = LbHelper<OrigList, 0, 0>::result();
  static const u64 result = LbHelper<OrigList, init, 1>::result();
};

// Compute L_a_star
template <typename TList, u32 i>
struct Numerator {
  static const u32 wcet = TList::Head::wcet;
  static const u32 deadline = TList::Head::deadline;
  static const u32 period = TList::Head::period;
  static constexpr double value = (static_cast<double>(wcet) / period) * (period - deadline) + Numerator<typename TList::Tail, i - 1>::value;
};

template <>
struct Numerator<NullType, 0> {
  static constexpr double value = 0.0;
};

template <typename TList, u32 i>
struct TotalUtilization {
  static const u32 wcet = TList::Head::wcet;
  static const u32 period = TList::Head::period;
  static constexpr double value = (static_cast<double>(wcet) / period) + TotalUtilization<typename TList::Tail, i - 1>::value;
};

template <>
struct TotalUtilization<NullType, 0> {
  static constexpr double value = 0.0;
};

template <typename TList, u32 i>
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

template <typename TList>
struct LaStar {
  static const u32 n = TL::Length<TList>::value;
  static constexpr double total_util = TotalUtilization<TList, n>::value;
  static const u64 la_upperbound = total_util < 1.0 ? static_cast<u64>(Numerator<TList, n>::value / (1.0 - TotalUtilization<TList, n>::value)) : ULLONG_MAX;
  static const int la_upperbound_2 = LaStarHelper<TList, n>::result;

  static const u64 result = la_upperbound_2 < 0 ? la_upperbound : (la_upperbound_2 > la_upperbound ? static_cast<u64>(la_upperbound_2) : la_upperbound);
};

template <typename TList>
struct L {
  static const u64 result = LaStar<TList>::result < Lb<TList>::result ? LaStar<TList>::result : Lb<TList>::result;
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
  static constexpr bool edge_case() {
    if constexpr (length < deadline) {
      return false;
    } else if constexpr (length <= period) {
      return true;
    } else {
      return DmaxHelper<period, deadline, length - period>::edge_case();
    }
  }

  static constexpr u64 remain() {
    if constexpr (length < deadline || length <= period) {
      return length;
    } else {
      return DmaxHelper<period, deadline, length - period>::remain();
    }
  }

  static const u64 result = edge_case() ? (length - remain() + deadline) : (length - remain() - period + deadline);
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

template <typename OrigList, u64 len>
struct Dmax<OrigList, NullType, len> {
  static const u64 result = 0;
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
