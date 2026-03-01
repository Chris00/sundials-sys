#include <nvector/nvector_serial.h>
#include <sunlinsol/sunlinsol_band.h>
#include <sunlinsol/sunlinsol_spbcgs.h>
#include <sunlinsol/sunlinsol_spgmr.h>
#include <sunlinsol/sunlinsol_dense.h>
#include <sunlinsol/sunlinsol_pcg.h>
#include <sunlinsol/sunlinsol_spfgmr.h>
#include <sunlinsol/sunlinsol_sptfqmr.h>
#include <sunmatrix/sunmatrix_band.h>
#include <sunmatrix/sunmatrix_dense.h>
#include <sunmatrix/sunmatrix_sparse.h>
#include <sunnonlinsol/sunnonlinsol_fixedpoint.h>
#include <sunnonlinsol/sunnonlinsol_newton.h>

#if USE_OPENMP
#include <nvector/nvector_openmp.h>
#endif

#if USE_ARKODE
#include <arkode/arkode.h>
#include <arkode/arkode_arkstep.h>       // ARKStep provides explicit, implicit, IMEX additive RK methods.
#include <arkode/arkode_erkstep.h>       // ERKStep provides explicit RK methods.
#include <arkode/arkode_mristep.h>       // MRIStep provides multirate RK methods.
#include <arkode/arkode_sprkstep.h>      // SPRKStep provides symplectic partitioned RK methods.
#include <arkode/arkode_bbdpre.h>        // BBDPre provides the band-block-diagonal preconditioner.
#include <arkode/arkode_bandpre.h>       // BANDPre provides the band preconditioner.
#endif

#if USE_CVODE & !USE_CVODES
#include <cvode/cvode.h>
#endif

#if USE_CVODES
#include <cvodes/cvodes.h>
#endif

#if USE_IDA
#include <ida/ida.h>
#endif

#if USE_IDAS
#include <idas/idas.h>
#endif

#if USE_KINSOL
#include <kinsol/kinsol.h>
#endif

#if USE_KLU
#include <sunlinsol/sunlinsol_klu.h>
#endif
