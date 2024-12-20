#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    improper_ctypes,
    clippy::all
)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// The communicator type changed from version 6 to 7.

/// Communicator connection type.
#[cfg(all(sundials_version_major = "6", not(feature="nvecopenmp")))]
pub type SUNComm = *mut std::ffi::c_void;

/// Create a new communicator type when MPI is not enabled.
#[cfg(all(sundials_version_major = "6", not(feature="nvecopenmp")))]
pub fn comm_no_mpi() -> SUNComm { std::ptr::null_mut() }

/// Create a new communicator type when MPI is not enabled.
#[cfg(all(sundials_version_major = "7", not(feature="nvecopenmp")))]
pub fn comm_no_mpi() -> SUNComm { std::ptr::null_mut() }

/// Backward compatibility type.
#[cfg(sundials_version_major = "7")]
pub type realtype = sunrealtype; // namespaced, so no prefix needed.


#[cfg(test)]
mod tests {
    use crate::*;
    use core:: {ffi::c_void, ptr};

    #[test]
    // This just tests if the most basic of all programs works. More tests to come soon.
    fn simple_ode() {
        unsafe extern "C" fn rhs(
            _t: f64,
            y: N_Vector,
            dy: N_Vector,
            _user_data: *mut c_void,
        ) -> i32 {
            *N_VGetArrayPointer(dy) = -*N_VGetArrayPointer(y);
            0
        }

        unsafe {
            let mut ctx = ptr::null_mut();
            if SUNContext_Create(comm_no_mpi(), &mut ctx) < 0 {
                panic!("Could not initialize Context.");
            }
            let y = N_VNew_Serial(1, ctx);
            *N_VGetArrayPointer(y) = 1.0;

            let mut cvode_mem = CVodeCreate(CV_ADAMS, ctx);

            CVodeInit(cvode_mem, Some(rhs), 0.0, y);
            CVodeSStolerances(cvode_mem, 1e-6, 1e-8);

            let matrix = SUNDenseMatrix(1, 1, ctx);
            let solver = SUNLinSol_Dense(y, matrix, ctx);

            CVodeSetLinearSolver(cvode_mem, solver, matrix);

            let mut t = 0f64;
            CVode(cvode_mem, 1.0, y, &mut t, CV_NORMAL);
            // y[0] is now exp(-1)

            let result = (*N_VGetArrayPointer(y) * 1e6) as i32;
            assert_eq!(result, 367879);

            N_VDestroy(y);
            CVodeFree(&mut cvode_mem);
            SUNLinSolFree(solver);
            SUNMatDestroy(matrix);
        }
    }
}
