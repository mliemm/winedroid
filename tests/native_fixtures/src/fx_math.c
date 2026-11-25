#define _GNU_SOURCE 1
#include <stdint.h>
typedef unsigned int __uint32_t;
#include <fenv.h>
#include <math.h>
#include <stdint.h>
#include <string.h>

extern int signgam;

double fx_sin(double value) { return sin(value); }
double fx_pow(double base, double exponent) { return pow(base, exponent); }
double fx_atan2(double y, double x) { return atan2(y, x); }
double fx_hypot(double x, double y) { return hypot(x, y); }
float fx_sqrtf(float value) { return sqrtf(value); }
float fx_powf(float base, float exponent) { return powf(base, exponent); }
int fx_ilogb(double value) { return ilogb(value); }
int fx_ilogbf(float value) { return ilogbf(value); }
long fx_lrint(double value) { return lrint(value); }
long long fx_llround(double value) { return llround(value); }
double fx_round(double value) { return round(value); }
double fx_nearbyint(double value) { return nearbyint(value); }
double fx_fma(double a, double b, double c) { return fma(a, b, c); }
double fx_ldexp(double value, int exponent) { return ldexp(value, exponent); }
double fx_scalbln(double value, long exponent) { return scalbln(value, exponent); }
double fx_fmax(double a, double b) { return fmax(a, b); }

double fx_frexp(double value, int *exponent) { return frexp(value, exponent); }
double fx_modf(double value, double *whole) { return modf(value, whole); }

double fx_nan(const char *text) { return nan(text); }
float fx_nanf(const char *text) { return nanf(text); }

double fx_lgamma(double value, int *sign)
{
    double result = lgamma(value);
    *sign = signgam;
    return result;
}

double fx_lgamma_r(double value, int *sign) { return lgamma_r(value, sign); }

void fx_sincos(double value, double *sine, double *cosine) { sincos(value, sine, cosine); }

int fx_classify(double value) { return fpclassify(value); }
int fx_isnan(double value) { return isnan(value); }
int fx_isinf(double value) { return isinf(value); }
int fx_signbit(double value) { return signbit(value); }

long double fx_fabsl(long double value) { return fabsl(value); }
long double fx_copysignl(long double magnitude, long double sign) { return copysignl(magnitude, sign); }

int fx_round_trip(int mode)
{
    int previous = fegetround();
    int accepted = fesetround(mode);
    int current = fegetround();
    fesetround(previous);
    return accepted * 100 + current;
}

int fx_round_invalid(void) { return fesetround(7); }

double fx_round_effect(int mode, double value)
{
    int previous = fegetround();
    double result;
    fesetround(mode);
    result = nearbyint(value);
    fesetround(previous);
    return result;
}

int fx_exception_flags(void)
{
    volatile double zero = 0.0;
    volatile double result;
    feclearexcept(FE_ALL_EXCEPT);
    result = 1.0 / zero;
    (void)result;
    return fetestexcept(FE_ALL_EXCEPT);
}

int fx_raise_all(void)
{
    feclearexcept(FE_ALL_EXCEPT);
    feraiseexcept(0xffffffff);
    return fetestexcept(0xffffffff);
}

int fx_env_round_trip(void)
{
    fenv_t saved;
    int result;
    fegetenv(&saved);
    fesetround(FE_DOWNWARD);
    result = fegetround();
    fesetenv(&saved);
    return result * 100 + fegetround();
}
