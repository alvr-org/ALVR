#include "pose.hpp"

#include <cmath>
#include <string.h>

#define UNW_LOCAL_ONLY
#include <libunwind.h>

namespace {

inline HmdMatrix34_t transposeMul33(const HmdMatrix34_t& a) {
  HmdMatrix34_t result;
  for (unsigned i = 0; i < 3; i++) {
    for (unsigned k = 0; k < 3; k++) {
      result.m[i][k] = a.m[k][i];
    }
  }
  result.m[0][3] = a.m[0][3];
  result.m[1][3] = a.m[1][3];
  result.m[2][3] = a.m[2][3];
  return result;
}


inline HmdMatrix34_t matMul33(const HmdMatrix34_t& a, const HmdMatrix34_t& b) {
  HmdMatrix34_t result;
  for (unsigned i = 0; i < 3; i++) {
    for (unsigned j = 0; j < 3; j++) {
      result.m[i][j] = 0.0f;
      for (unsigned k = 0; k < 3; k++) {
        result.m[i][j] += a.m[i][k] * b.m[k][j];
      }
    }
  }
  return result;
}


bool check_pose(const TrackedDevicePose_t & p)
{
  if (p.bPoseIsValid != 1 or p.bDeviceIsConnected != 1)
    return false;

  if (p.eTrackingResult != 200)
    return false;

  auto m = matMul33(p.mDeviceToAbsoluteTracking, transposeMul33(p.mDeviceToAbsoluteTracking));
  for (int i = 0 ; i < 3; ++i )
  {
    for (int j = 0 ; j < 3 ; ++j)
    {
      if (std::abs(m.m[i][j] - (i == j)) > 0.1)
        return false;
    }
  }
  return true;
}

}

// For a smooth experience, the correct pose for a frame must be known.
// Of course this is not part of vulkan parameters, so we must inspect
// the stack.
// First we look for the correct function (CRenderThread::UpdateAsync),
// then we scan all the local variables, and check for a suitable one.
// Such a variable is a TrackedDevicePose_t, with both booleans to true,
// which we compare to 1 to avoid false positives, a tracking result of
// 200, and a rotation matrix (A*transpose(A)) close to identity.
const TrackedDevicePose_t & find_pose_in_call_stack()
{
  static TrackedDevicePose_t * res;
  if (res != nullptr)
    return *res;
  static TrackedDevicePose_t notfound;
  unw_context_t ctx;
  unw_getcontext(&ctx);
  unw_cursor_t cursor;
  unw_init_local(&cursor, &ctx);
  while (unw_step(&cursor) > 0)
  {
    char name[1024];
    unw_word_t off;
    unw_get_proc_name(&cursor, name, sizeof(name), &off);
    if ((strcmp("_ZN13CRenderThread11UpdateAsyncEv", name) == 0) || (strcmp("_ZN13CRenderThread6UpdateEv", name) == 0))
    {
      unw_word_t sp, sp_end;
      unw_get_reg(&cursor, UNW_REG_SP, &sp);
      unw_step(&cursor);
      unw_get_reg(&cursor, UNW_REG_SP, &sp_end);
      for (uintptr_t addr = sp ; addr < sp_end; addr += 4)
      {
        TrackedDevicePose_t * p = (TrackedDevicePose_t *) addr;
        if (check_pose(*p))
        {
          res = p;
          return *p;
        }
      }
      return notfound;
    }
  }
  return notfound;
}
