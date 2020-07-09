#include "animation_curve.h"

using namespace std;
using namespace std::chrono;

AnimationCurve::AnimationCurve(function<float(float)> curve,
                               duration<float> duration) {
    mCurve = curve;
    mDuration = duration;
    mStartInstant = steady_clock::now();
}

void AnimationCurve::Start(steady_clock::time_point instant, float startValue,
                           float endValue) {
    mStartValue = startValue;
    mEndValue = endValue;
    mStartInstant = instant;
}

float AnimationCurve::GetValue(steady_clock::time_point instant) {
    float normalizedTime = (instant - mStartInstant) / mDuration;
    float value = mCurve(normalizedTime);

    return mStartValue + value * (mEndValue - mStartValue);
}
