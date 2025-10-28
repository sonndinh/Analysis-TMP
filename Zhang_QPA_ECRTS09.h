#include <loki/Typelist.h>

#include <climits>
#include <algorithm>

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
  static const i32 deadline = static_cast<i32>(TList::Head::deadline);
  static const i32 period = static_cast<i32>(TList::Head::period);
  static constexpr double util = static_cast<double>(wcet) / period;
  static constexpr double value = (period - deadline) * util + Numerator<typename TList::Tail, i - 1>::value;
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
  static const i32 deadline = static_cast<i32>(TList::Head::deadline);
  static const i32 period = static_cast<i32>(TList::Head::period);
  static const i32 my_delta = deadline - period;
  static const i32 result = std::max(my_delta, LaStarHelper<typename TList::Tail, i - 1>::result);
};

template <>
struct LaStarHelper<NullType, 0> {
  static const i32 result = 0;
};

template <typename TList>
struct LaStar {
  static const u32 n = TL::Length<TList>::value;
  static constexpr double total_util = TotalUtilization<TList, n>::value;
  static const i64 la_upperbound = total_util < 1.0 ? static_cast<i64>(Numerator<TList, n>::value / (1.0 - total_util)) : LLONG_MAX;
  static const i64 la_upperbound_2 = LaStarHelper<TList, n>::result;

  // Safe to cast to u64 since the result of std::max should be non-negative
  static const u64 result = static_cast<u64>(std::max(la_upperbound, la_upperbound_2));
};

template <typename TList>
struct L {
  static const u32 n = TL::Length<TList>::value;
  static constexpr double total_util = TotalUtilization<TList, n>::value;
  static const u64 result = total_util < 1.0 ? std::min(LaStar<TList>::result, Lb<TList>::result) : Lb<TList>::result;
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

// Compute maximum absolute deadline that is less than t
template <typename OrigList, typename RemainList, u64 t>
struct Dmax {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static const u32 period = RemainList::Head::period;
  static const u32 deadline = RemainList::Head::deadline;

  static constexpr u64 result() {
    if (deadline < t) {
      const u64 tmp = ((t - deadline) / period) * period + deadline;
      const u64 last_deadline = tmp < t ? tmp : (tmp - period);
      return std::max(last_deadline, Dmax<OrigList, typename RemainList::Tail, t>::result());
    } else {
      return Dmax<OrigList, typename RemainList::Tail, t>::result();
    }
  }
};

template <typename OrigList, u64 t>
struct Dmax<OrigList, NullType, t> {
  static constexpr u64 result() {
    return 0;
  }
};

// QPA test
template <typename  OrigList, u64 t>
struct QPAHelper {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static const u64 h_t = Pdf<OrigList, t, n>::result;
  static const u32 d_min = Dmin<OrigList>::result;

  // Update the value of t and recur
  static const bool keep_going = h_t <= t && h_t > d_min;
  static const u64 new_t = keep_going ? (h_t < t ? h_t : Dmax<OrigList, OrigList, t>::result()) : 0;

  static constexpr bool result() {
    if constexpr (!keep_going) {
      return h_t <= d_min ? true : false;
    } else {
      return QPAHelper<OrigList, new_t>::result();
    }
  }
};

template <typename OrigList>
struct QPA {
  static const u32 n = static_cast<u32>(TL::Length<OrigList>::value);
  static constexpr double total_util = TotalUtilization<OrigList, n>::value;
  static const u64 t = Dmax<OrigList, OrigList, L<OrigList>::result>::result();

  static constexpr bool schedulable() {
    if constexpr (total_util > 1.0) {
      return false;
    } else {
      return QPAHelper<OrigList, t>::result();
    }
  }
};
