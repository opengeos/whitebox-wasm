use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::sync::Arc;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::color_math::{hsi2value, value2hsi, value2i};
use wbraster::{DataType, Raster, RasterConfig, RasterFormat};

use super::color_support;
use crate::memory_store;

pub struct FastAlmostGaussianFilterTool;
pub struct EdgePreservingMeanFilterTool;
pub struct UnsharpMaskingTool;
pub struct DiffOfGaussiansFilterTool;
pub struct AdaptiveFilterTool;
pub struct LeeFilterTool;
pub struct LeeSigmaFilterTool;
pub struct RefinedLeeFilterTool;
pub struct EnhancedLeeFilterTool;
pub struct ConservativeSmoothingFilterTool;
pub struct OlympicFilterTool;
pub struct KNearestMeanFilterTool;
pub struct HighPassMedianFilterTool;
pub struct LaplacianOfGaussiansFilterTool;

#[derive(Clone, Copy)]
enum Phase3Op {
    FastAlmostGaussian,
    EdgePreservingMean,
    Unsharp,
    DiffOfGaussians,
    Adaptive,
    Lee,
    LeeSigma,
    RefinedLee,
    EnhancedLee,
    ConservativeSmoothing,
    Olympic,
    KNearestMean,
    HighPassMedian,
    LaplacianOfGaussians,
}

impl Phase3Op {
    fn id(self) -> &'static str {
        match self {
            Self::FastAlmostGaussian => "fast_almost_gaussian_filter",
            Self::EdgePreservingMean => "edge_preserving_mean_filter",
            Self::Unsharp => "unsharp_masking",
            Self::DiffOfGaussians => "diff_of_gaussians_filter",
            Self::Adaptive => "adaptive_filter",
            Self::Lee => "lee_filter",
            Self::LeeSigma => "lee_sigma_filter",
            Self::RefinedLee => "refined_lee_filter",
            Self::EnhancedLee => "enhanced_lee_filter",
            Self::ConservativeSmoothing => "conservative_smoothing_filter",
            Self::Olympic => "olympic_filter",
            Self::KNearestMean => "k_nearest_mean_filter",
            Self::HighPassMedian => "high_pass_median_filter",
            Self::LaplacianOfGaussians => "laplacian_of_gaussians_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::FastAlmostGaussian => "Fast Almost Gaussian Filter",
            Self::EdgePreservingMean => "Edge Preserving Mean Filter",
            Self::Unsharp => "Unsharp Masking",
            Self::DiffOfGaussians => "Difference of Gaussians Filter",
            Self::Adaptive => "Adaptive Filter",
            Self::Lee => "Lee Filter",
            Self::LeeSigma => "Lee Sigma Filter",
            Self::RefinedLee => "Refined Lee Filter",
            Self::EnhancedLee => "Enhanced Lee Filter",
            Self::ConservativeSmoothing => "Conservative Smoothing Filter",
            Self::Olympic => "Olympic Filter",
            Self::KNearestMean => "K-Nearest Mean Filter",
            Self::HighPassMedian => "High-Pass Median Filter",
            Self::LaplacianOfGaussians => "Laplacian of Gaussians Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::FastAlmostGaussian => r#"The Fast Almost Gaussian filter provides rapid Gaussian-approximation smoothing via iterative separable box (averaging) filtering, achieving near-Gaussian blur response with O(N) computational complexity regardless of blur radius. Implementation applies successive box convolutions (each computing local pixel averages) to approximate cumulative Gaussian distribution; N iterations approximate increasingly larger Gaussian kernels. The mathematical basis uses the central limit theorem: repeated convolution of box functions approaches Gaussian distribution asymptotically. Key features include computational speed enabling large-kernel smoothing on big imagery, separable implementation reducing memory requirements, parameter control via iteration count (controls blur radius), and effectiveness on any data type. Fast Almost Gaussian filtering excels in rapid image pyramids for multi-scale analysis, real-time satellite imagery browsing, preprocessing enormous remote sensing datasets before classification, and interactive image viewers requiring responsive smoothing. Output interpretation shows that iteration count directly relates to blur radius: N=1 produces minimal smoothing; N=3-5 provides moderate blur; N>10 creates strong smoothing approximating large-kernel Gaussians. Output values progressively shift toward local mean as iterations increase; variance reduction follows predictable patterns. Remaining values stay within input data ranges. Comparison with true Gaussian filtering shows acceptable approximation within 2-5% error for most applications. Speed improvement over true Gaussian filtering increases dramatically at large radii: 5-20× faster for radius >20 pixels. Minor artifacts include subtle waviness along edges (box artifacts accumulating across iterations) and slightly different boundary handling than Gaussian. Monitor output histograms for multimodal distributions indicating sufficient smoothing. Apply in image preprocessing pipelines where speed justifies minor approximation errors."#,
            Self::EdgePreservingMean => r#"The Edge-Preserving Mean filter performs selective pixel averaging by computing local means while excluding outlier pixels that likely represent edges or noise. Implementation sorts pixel neighborhoods, removes extreme values (lowest and highest, or values exceeding statistical threshold), and averages remaining values. This robust approach balances smoothing against sharpness preservation. Variants include weighted averaging emphasizing center pixels and adaptive threshold selection based on local statistics. Key features include simple parameter control (exclusion count or threshold), computational efficiency via sorting small neighborhoods, effective noise reduction without detail blurring, and applicability to optical, radar, and thermal data. Edge-Preserving Mean filtering excels in optical satellite preprocessing for vegetation index calculation (removes shadows and clouds without blurring features), DEM smoothing preserving slope breaks, thermal image denoising maintaining boundary sharpness, and orthophoto preparation for manual digitization. Output interpretation reveals that removed outliers concentrate at edges and noise regions; remaining values average creating local smoothing. Exclusion parameters control edge sharpness: excluding single extreme removes salt-pepper noise; excluding multiple extremes produces smoother results with gentler edge transition. Output ranges remain within input; statistics shift toward inlier means. Monitor output difference from input to identify filtered regions (typically high-variance areas). Edge preservation quality depends on outlier identification accuracy; verify visually that features remain sharp. Common artifacts include inadequate smoothing if thresholds are too strict and excessive filtering if exclusion counts are too high. Iteration count enables progressive filtering; single pass provides gentle smoothing. Apply before threshold-based classification to reduce noise-driven category misclassification while maintaining feature boundaries."#,
            Self::Unsharp => r#"Unsharp masking performs image sharpening by subtracting a smoothed (low-pass) version from the original image, enhancing edges and fine details. The mathematical transformation is I_sharp = I_original + w·(I_original - I_smooth), where w is sharpening weight controlling enhancement strength. The process isolates high-frequency components and amplifies them, effectively separating detail from broad tonal variation. Key features include flexible parameter control (blur radius and weight enabling detail control), computational efficiency via Gaussian smoothing reuse, interpretable enhancement (weight=0 gives original; weight=1 gives true high-pass; weight>1 provides aggressive sharpening), and effectiveness on multispectral data. Unsharp masking excels in satellite image preparation for manual interpretation (enhances subtle terrain, vegetation, infrastructure), LiDAR-derived product enhancement (sharpens DEMs, vegetation metrics), orthophoto quality improvement for feature visibility, and archaeological/survey imagery enhancement. Output interpretation requires understanding that sharpened values concentrate on edges; homogeneous regions remain unchanged. Weight parameter controls enhancement magnitude: w=0.5-1.0 provides subtle enhancement; w=1.0-2.0 provides moderate sharpening; w>2.0 produces aggressive, potentially artifact-laden results. Blur radius controls feature scale: smaller radius (3-5 pixels) enhances fine texture; larger radius (10-20 pixels) enhances moderate features. Output values may exceed input range; clipping typically necessary. Artifacts include halos around strong edges (larger weight or radius = more pronounced), amplified noise if source is noisy, and potential false colors in multispectral sharpening. Verify enhancement via difference images. Apply selectively in visualization workflows; avoid before automated analysis that's sensitive to output range changes."#,
            Self::DiffOfGaussians => r#"Difference of Gaussians (DoG) filtering detects edges and fine features via subtraction of two Gaussian-blurred versions with different radii, creating a bandpass filter emphasizing intermediate spatial frequencies. Implementation computes two Gaussian blurs (small radius σ₁ and large radius σ₂), then subtracts: DoG = G(σ₁) - G(σ₂). This non-linear combination enhances edges and ridges while suppressing both fine noise and broad illumination trends. Key features include tunable frequency response (ratio σ₂/σ₁ controls bandpass characteristics), zero-centered output (bipolar: positive and negative values), applicability to edge detection and feature extraction, and computational efficiency via Gaussian reuse. Difference of Gaussians excels in geological structure detection (faults, lineaments appear as high DoG response), building extraction from orthophotos, fine-texture enhancement in remote sensing mosaics, and neuroscience-inspired processing models. Output interpretation shows that edges produce peak responses (positive on bright side, negative on dark side) with zero-crossing precisely at transitions. Zero-crossing detection reveals true edges independent of edge direction. DoG magnitude indicates edge strength; typical range spans -1000 to +1000 for 8-bit input depending on local contrast. Ratio σ₂/σ₁ determines bandpass center: ratio~2 emphasizes very fine features; ratio~5 emphasizes intermediate features; ratio~10 emphasizes coarser features. Bimodal output distributions (peaks at positive and negative extremes) indicate good edge separation. Common artifacts include halos around strong edges and potential under-response if frequency band doesn't match feature scale. Combine with magnitude thresholding for edge extraction. Apply before morphological post-processing for robust feature extraction from noisy satellite imagery."#,
            Self::Adaptive => r#"The Adaptive Filter adjusts filtering strength dynamically based on local image statistics (mean, variance, kurtosis), enabling context-aware smoothing that responds to scene characteristics. Implementation partitions images into moving windows, computes local statistics (detecting noise-dominated versus feature-dominated regions), and selects filter parameters accordingly. Flat regions smooth aggressively; complex regions filter gently. Mathematical basis uses statistical tests to identify local character: regions with variance below threshold smooth heavily; regions exceeding threshold preserve detail. Key features include automatic parameter adaptation (user specifies ranges; algorithm selects locally), applicability to any filter kernel (Gaussian, median, morphological), and effectiveness on optical and radar imagery. Adaptive filtering excels in preprocessing heterogeneous satellite mosaics (different sensors, acquisition conditions), LiDAR point-cloud smoothing (preserves vegetation edges in forests; smooths ground in open areas), selective SAR speckle reduction respecting both targets and background, and multi-temporal image stacking. Output interpretation reveals that homogeneous regions undergo intensive filtering (low local variance → strong smoothing); complex regions filter conservatively (high variance → minimal processing). Smoothing radius dynamically adjusts per-region: flat areas receive large-radius filtering; textured areas receive small-radius or no filtering. Output ranges remain within input; examine spatial filtering-strength map to validate adaptation. Statistics shift toward local means in smooth regions; complex regions remain largely unchanged. Common artifacts include potential over-smoothing at region boundaries (smooth transition typically applied) and sensitivity to noise-variance relationships. Verify adaptation by analyzing local variance maps. Apply in automated preprocessing pipelines handling multi-source data where uniform filtering insufficient."#,
            Self::Lee => r#"The Lee filter reduces speckle in SAR backscatter with Lee's minimum mean-square-error (MMSE) estimator under the multiplicative speckle model I = S·N. For each pixel it computes the mean μ and standard deviation σ of the moving window, the local coefficient of variation Ci = σ/μ and the speckle coefficient of variation Cu = 1/√ENL, then returns F = μ + w·(I − μ) with w = clamp(1 − Cu²/Ci², 0, 1). In homogeneous areas, where the local variation is no more than speckle (Ci ≤ Cu), the pixel is replaced by the window mean; across edges, point targets and texture (Ci ≫ Cu) w approaches 1 and the pixel is kept. Because only the ratio σ/μ enters the weight, the result is scale-invariant: linear power (sigma0, gamma0), amplitude and scaled digital numbers give the same relative result. Input should be linear (not dB) backscatter with positive values; windows whose mean is not positive are left unfiltered. The 'enl' parameter is the equivalent number of looks of the input: 1 for single-look intensity, higher for multilooked products (e.g. 4 or more for GRD, NISAR GCOV or OPERA RTC); a larger ENL means less speckle is assumed and less smoothing is applied. Window size trades smoothing in homogeneous areas against edge blur. The default is 11×11 for consistency with the other filters; 5×5 or 7×7 is usual for SAR and keeps edges and small features sharper. For the classic Lee sigma filter with an absolute intensity range, use lee_sigma_filter."#,
            Self::LeeSigma => r#"The Lee sigma filter averages, for each pixel, only those neighbours whose values lie within ±sigma of the centre value: F = mean(I_j : |I_j − I_center| ≤ sigma). If fewer than m_value neighbours qualify, the pixel is replaced by the mean of its eight immediate neighbours, which removes isolated spikes. The range is absolute, in the units of the data, so this filter suits data whose noise is additive, such as SAR backscatter in decibels (where a sigma of about 1-3 dB is typical) or DEMs. On linear SAR backscatter (values mostly below 1) the default sigma of 10 includes every neighbour and the filter becomes a plain mean; use lee_filter or enhanced_lee_filter for linear backscatter instead."#,
            Self::RefinedLee => r#"The Refined Lee filter reduces SAR speckle while keeping edges by choosing, for each pixel, the most homogeneous of the nine overlapping 3×3 sub-windows that contain it within the surrounding 5×5 neighbourhood (the sub-window with the lowest coefficient of variation σ/μ) and returning that sub-window's mean. Because the chosen sub-window usually lies on the same side of an edge as the pixel, edges stay sharp, while uniform areas are smoothed over nine cells. The neighbourhood is fixed at 5×5; there is no window-size parameter. Speckle reduction is therefore moderate; use lee_filter or enhanced_lee_filter with a larger window for stronger smoothing of homogeneous areas. Input should be linear (not dB) backscatter with positive values."#,
            Self::EnhancedLee => r#"The Enhanced Lee filter (Lopes et al., 1990) reduces SAR speckle and treats homogeneous areas, heterogeneous areas and point targets differently. For each pixel it computes the window mean μ and standard deviation σ, the local coefficient of variation Ci = σ/μ, the speckle coefficient of variation Cu = 1/√ENL and the upper limit Cmax = √(1 + 2/ENL). Where Ci ≤ Cu (homogeneous, speckle only) the pixel is replaced by the mean μ. Where Ci ≥ Cmax (a point target or strong edge) the pixel is kept unchanged. In between, F = μ·W + I·(1 − W) with W = exp(−(Ci − Cu)/(Cmax − Ci)), so smoothing fades out as heterogeneity rises. Because only σ/μ enters the weights the result is scale-invariant: linear power, amplitude and scaled digital numbers give the same relative result. Input should be linear (not dB) backscatter with positive values; windows whose mean is not positive are left unfiltered. 'enl' is the equivalent number of looks of the input (1 for single-look intensity; higher for multilooked products)."#,
            Self::ConservativeSmoothing => r#"Conservative Smoothing implements non-linear filtering by replacing each pixel with the average of similar neighbors (within a defined intensity range), preserving edges while reducing noise. Implementation examines neighborhoods, identifies pixels within intensity threshold of central pixel, and averages these similar-value pixels. Mathematical formulation: F = (1/N) Σ(I_j : |I_j - I_center| < T), where T is similarity threshold and N is count of similar pixels. Key features include simple threshold-based similarity definition, effectiveness on optical and radar imagery, parameter interpretability (threshold controls edge-sharpness), and minimal computational overhead. Conservative Smoothing excels in multispectral satellite image preprocessing (reduces noise while preserving spectral boundaries), thermal image enhancement (smooths radiometric noise while maintaining temperature discontinuities), LiDAR classification smoothing (preserves vegetation/ground boundaries), and noisy survey data preprocessing. Output interpretation shows that homogeneous regions average completely (all neighbors similar); edges filter minimally (similar pixels only on same side). Threshold parameter controls edge preservation: small threshold (strict similarity) produces mild smoothing; large threshold (loose similarity) produces aggressive smoothing potentially losing edges. Output values exactly match input neighbor values (no interpolation; output is average of existing values). Statistics shift toward local clusters; heterogeneous regions show minimal change. Verify threshold effectiveness via visual inspection and local histogram analysis. Common artifacts include insufficient smoothing if thresholds are too strict and edge blurring if thresholds are too loose. Iteration count enables progressive filtering: single pass provides gentle smoothing; multiple passes intensify effect. Apply before classification where noise-driven category confusion must be reduced while class boundaries remain sharp."#,
            Self::Olympic => r#"The Olympic Filter implements rank-based smoothing by removing the single highest and single lowest values from each pixel neighborhood, then averaging the remaining pixels. This robust filtering approach eliminates extreme outliers (likely noise or spurious values) while preserving the central tendency. Mathematical formulation: F = (1/(N-2)) Σ(I_sorted[2:N-1]), where sorted neighborhood values exclude highest and lowest. Implementation requires sorting small neighborhoods (computationally efficient) but produces effective noise reduction. Key features include simple outlier removal strategy, effective salt-pepper noise reduction, edge-aware filtering (edges often appear as extremes), and applicability to any data type. Olympic filtering excels in optical satellite imagery preprocessing (removes isolated bright cloud pixels and dark shadows), DEM smoothing reducing survey noise artifacts, thermal image denoising (removes sensor outliers), and radar image preprocessing. Output interpretation reveals that symmetric noise distributions (equal numbers of high/low outliers) filter symmetrically; skewed distributions (more highs or lows) produce directional filtering. Neighborhood size controls smoothing extent: 3×3 window removes 2 extremes from 9 pixels (mild filtering); larger windows filter more aggressively. Output values remain within input range (output is average of actual neighborhood values). Statistics show controlled variance reduction targeting outliers specifically. Difference images (original - filtered) highlight removed outliers; concentrated high-value regions indicate effective noise isolation. Common artifacts include insufficient smoothing if outlier frequency is low and edge blurring if edge pixels consistently rank as extremes. Multiple iterations enable progressive smoothing: single pass provides noise reduction; repeated passes intensify smoothing. Apply in rapid noise-reduction workflows requiring simple, interpretable filtering, particularly effective for salt-pepper noise in multisensor mosaics."#,
            Self::KNearestMean => r#"Performs edge-preserving k-nearest neighbor mean smoothing: sorts neighborhood by distance to center value, averages k closest values. Hybrid approach preserving edges via similarity weighting. Center pixel and k-1 most similar neighbors are averaged. More sophisticated than simple k-NN (considers both spatial and intensity similarity). Computationally efficient relative to bilateral. K-nearest approach adaptively selects neighbors: pixel values close to center are averaged, dissimilar pixels ignored. K parameter controls smoothing: k=1 (no smoothing), k=n (all neighbors = mean filter). Typically k=n/2 (half the neighborhood). Efficient alternative to bilateral filter—similar edge preservation at lower computational cost. Particularly useful for images with strong intensity discontinuities. Applications: (1) Edge-preserving smoothing (alternative to bilateral), (2) Fast preprocessing for classification, (3) Efficiency-critical preprocessing, (4) Multi-band image filtering. Typical parameters: k=neighborhood_size/2 to 3/4."#,
            Self::HighPassMedian => r#"Performs high-pass filtering by subtracting local median from center values: output = pixel - median_neighborhood. Combines high-pass filtering (enhances detail) with median robustness (removes noise). Center-around-zero output (negative = darker than surroundings, positive = brighter). Robust to outliers compared to Gaussian-based high-pass. High-pass residual reveals local deviations from median trend. Particularly robust for noisy data—median is more stable than mean for outliers. Output emphasizes fine-scale variation. Often applied to Gaussian-smoothed versions (creates band-pass filter). Useful for texture enhancement and feature extraction from noisy imagery. Applications: (1) Texture enhancement from noisy data, (2) Detail extraction before classification, (3) Robust feature detection, (4) Preprocessing for texture-based segmentation, (5) SAR preprocessing."#,
            Self::LaplacianOfGaussians => r#"Performs Laplacian-of-Gaussians (LoG) edge enhancement combining Gaussian smoothing with Laplacian edge detection. Uses the analytic LoG kernel, truncated where the Gaussian weight falls below 0.001. First smooths image (reduces noise), then applies Laplacian (detects edges). Classical multi-scale edge detection technique. The output is the LoG response itself: zero over flat areas, with opposite signs on either side of an edge. Zero-crossing detection on LoG output identifies precise edge locations. LoG is standard for scale-space edge detection: Gaussian removes noise, Laplacian amplifies edges. Sigma parameter controls scale of edges detected: small sigma detects fine edges, large sigma detects broad edges. Multiple sigma values enable multi-scale edge detection (identify features at different scales). Applications: (1) Robust edge detection (noise-resistant), (2) Multi-scale edge detection (use multiple sigma), (3) Zero-crossing edge localization, (4) Preprocessing for segmentation, (5) Blob detection via LoG zero-crossings. Workflow: apply LoG→identify zero-crossings→trace edges→vectorization or further processing."#,
        }
    }
}

impl FastAlmostGaussianFilterTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn load_raster(path: &str) -> Result<Arc<Raster>, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_arc_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}': store entry is missing",
                    id
                ))
            });
        }

        Raster::read(path)
            .map(Arc::new)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
    }

    fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
        if let Some(output_path) = output_path {
            if let Some(parent) = output_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ToolError::Execution(format!("failed creating output directory: {e}"))
                    })?;
                }
            }

            let output_path_str = output_path.to_string_lossy().to_string();
            let output_format = RasterFormat::for_output_path(&output_path_str)
                .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
            output
                .write(&output_path_str, output_format)
                .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;
            Ok(output_path_str)
        } else {
            let id = memory_store::put_raster(output);
            Ok(memory_store::make_raster_memory_path(&id))
        }
    }

    fn metadata_for(op: Phase3Op) -> ToolMetadata {
        let mut params = vec![ToolParamSpec {
            name: "input",
            description: "Input raster path or typed raster object.",
            required: true,
        }];

        match op {
            Phase3Op::FastAlmostGaussian => {
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Approximate Gaussian sigma (>=1.8 recommended, default 1.8).",
                    required: false,
                });
            }
            Phase3Op::EdgePreservingMean => {
                params.push(ToolParamSpec {
                    name: "filter_size",
                    description: "Square neighborhood size in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "threshold",
                    description: "Maximum absolute neighbor difference to include in local mean (default 15.0).",
                    required: false,
                });
            }
            Phase3Op::Unsharp => {
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Gaussian sigma used by the blur mask (0.5-20.0, default 0.75).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "amount",
                    description: "Sharpening amount multiplier applied to residual (default 100.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "threshold",
                    description: "Minimum absolute residual to sharpen (default 0.0).",
                    required: false,
                });
            }
            Phase3Op::DiffOfGaussians => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Smaller Gaussian sigma (0.25-20.0, default 2.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "sigma2",
                    description: "Larger Gaussian sigma (0.5-20.0, default 4.0).",
                    required: false,
                });
            }
            Phase3Op::Adaptive => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "threshold",
                    description: "Absolute z-score threshold for mean replacement (default 2.0).",
                    required: false,
                });
            }
            Phase3Op::Lee => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "enl",
                    description: "Equivalent number of looks of the input; speckle coefficient of variation is 1/sqrt(enl) (default 1.0, single-look).",
                    required: false,
                });
            }
            Phase3Op::LeeSigma => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Absolute inclusion half-width around the center value, in data units (default 10.0).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "m_value",
                    description: "Minimum in-range sample count before fallback averaging (default 5.0).",
                    required: false,
                });
            }
            Phase3Op::RefinedLee => {}
            Phase3Op::EnhancedLee => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "enl",
                    description: "Equivalent number of looks of the input; sets Cu = 1/sqrt(enl) and Cmax = sqrt(1 + 2/enl) (default 4.0).",
                    required: false,
                });
            }
            Phase3Op::ConservativeSmoothing => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 3).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 3).",
                    required: false,
                });
            }
            Phase3Op::Olympic => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
            }
            Phase3Op::KNearestMean => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 3).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 3).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "k",
                    description: "Number of nearest neighbours to average (default 5).",
                    required: false,
                });
            }
            Phase3Op::HighPassMedian => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Neighborhood width in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Neighborhood height in pixels (odd integer, default 11).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "sig_digits",
                    description: "Significant digits used for histogram binning (default 2).",
                    required: false,
                });
            }
            Phase3Op::LaplacianOfGaussians => {
                params.push(ToolParamSpec {
                    name: "sigma",
                    description: "Gaussian sigma used by the LoG kernel (0.5-20.0, default 0.75).",
                    required: false,
                });
            }
        }

        params.push(ToolParamSpec {
            name: "output",
            description: "Optional output path. If omitted, output remains in memory.",
            required: false,
        });

        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
        }
    }

    fn manifest_for(op: Phase3Op) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));

        match op {
            Phase3Op::FastAlmostGaussian => {
                defaults.insert("sigma".to_string(), json!(1.8));
            }
            Phase3Op::EdgePreservingMean => {
                defaults.insert("filter_size".to_string(), json!(11));
                defaults.insert("threshold".to_string(), json!(15.0));
            }
            Phase3Op::Unsharp => {
                defaults.insert("sigma".to_string(), json!(0.75));
                defaults.insert("amount".to_string(), json!(100.0));
                defaults.insert("threshold".to_string(), json!(0.0));
            }
            Phase3Op::DiffOfGaussians => {
                defaults.insert("sigma1".to_string(), json!(2.0));
                defaults.insert("sigma2".to_string(), json!(4.0));
            }
            Phase3Op::Adaptive => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("threshold".to_string(), json!(2.0));
            }
            Phase3Op::Lee => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("enl".to_string(), json!(1.0));
            }
            Phase3Op::LeeSigma => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("sigma".to_string(), json!(10.0));
                defaults.insert("m_value".to_string(), json!(5.0));
            }
            Phase3Op::ConservativeSmoothing => {
                defaults.insert("filter_size_x".to_string(), json!(3));
                defaults.insert("filter_size_y".to_string(), json!(3));
            }
            Phase3Op::Olympic => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
            }
            Phase3Op::KNearestMean => {
                defaults.insert("filter_size_x".to_string(), json!(3));
                defaults.insert("filter_size_y".to_string(), json!(3));
                defaults.insert("k".to_string(), json!(5));
            }
            Phase3Op::HighPassMedian => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("sig_digits".to_string(), json!(2));
            }
            Phase3Op::LaplacianOfGaussians => {
                defaults.insert("sigma".to_string(), json!(0.75));
            }
            Phase3Op::RefinedLee => {}
            Phase3Op::EnhancedLee => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
                defaults.insert("enl".to_string(), json!(4.0));
            }
        }

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));

        let params = Self::metadata_for(op)
            .params
            .into_iter()
            .map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            })
            .collect();

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
            defaults,
            examples: vec![ToolExample {
                name: format!("basic_{}", op.id()),
                description: format!("Applies {} to an input raster.", op.id()),
                args: example_args,
            }],
            tags: vec![
                "remote_sensing".to_string(),
                "raster".to_string(),
                "filter".to_string(),
                op.id().to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Stable,
        }
    }

    fn build_gaussian_kernel(sigma: f64) -> (Vec<isize>, Vec<isize>, Vec<f64>) {
        let recip_root_2_pi_times_sigma_d = 1.0 / ((2.0 * PI).sqrt() * sigma);
        let two_sigma_sqr_d = 2.0 * sigma * sigma;

        let mut filter_size = 0usize;
        for i in 0..250usize {
            let dist2 = (i * i) as f64;
            let weight = recip_root_2_pi_times_sigma_d * (-dist2 / two_sigma_sqr_d).exp();
            if weight <= 0.001 {
                filter_size = i * 2 + 1;
                break;
            }
        }
        if filter_size % 2 == 0 {
            filter_size += 1;
        }
        if filter_size < 3 {
            filter_size = 3;
        }

        let num_filter = filter_size * filter_size;
        let midpoint = (filter_size as f64 / 2.0).floor() as isize;
        let mut dx = vec![0isize; num_filter];
        let mut dy = vec![0isize; num_filter];
        let mut weights = vec![0.0f64; num_filter];
        let mut weight_sum = 0.0f64;

        let mut a = 0usize;
        for row in 0..filter_size {
            for col in 0..filter_size {
                let x = col as isize - midpoint;
                let y = row as isize - midpoint;
                dx[a] = x;
                dy[a] = y;
                let w = recip_root_2_pi_times_sigma_d
                    * (-(x * x + y * y) as f64 / two_sigma_sqr_d).exp();
                weights[a] = w;
                weight_sum += w;
                a += 1;
            }
        }

        if weight_sum > 0.0 {
            for w in &mut weights {
                *w /= weight_sum;
            }
        }

        (dx, dy, weights)
    }

    fn gaussian_blur_values(input: &Raster, sigma: f64, packed_rgb: bool) -> Vec<Vec<f64>> {
        let (dx, dy, weights) = Self::build_gaussian_kernel(sigma);
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        (0..bands)
            .into_par_iter()
            .map(|band_idx| {
                let band = band_idx as isize;
                let mut out = vec![nodata; rows * cols];
                out.par_chunks_mut(cols).enumerate().for_each(|(row_idx, out_row)| {
                    let row = row_idx as isize;
                    for col_idx in 0..cols {
                        let col = col_idx as isize;
                        let z0_raw = input.get(band, row, col);
                        if input.is_nodata(z0_raw) {
                            continue;
                        }
                        let mut sum = 0.0f64;
                        let mut zf = 0.0f64;
                        for i in 0..dx.len() {
                            let zn_raw = input.get(band, row + dy[i], col + dx[i]);
                            if input.is_nodata(zn_raw) {
                                continue;
                            }
                            let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                            sum += weights[i];
                            zf += weights[i] * zn;
                        }
                        if sum > 0.0 {
                            out_row[col_idx] = zf / sum;
                        }
                    }
                });
                out
            })
            .collect()
    }

    fn write_values_into_output(
        input: &Raster,
        output: &mut Raster,
        values: &[Vec<f64>],
        packed_rgb: bool,
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut rows_buf = vec![vec![nodata; cols]; rows];

            rows_buf
                .par_iter_mut()
                .enumerate()
                .for_each(|(r, out_row)| {
                    for c in 0..cols {
                        let idx = r * cols + c;
                        let v = values[band_idx][idx];
                        if v == nodata {
                            continue;
                        }
                        if packed_rgb {
                            let z0 = input.get(band, r as isize, c as isize);
                            let (h, s, _) = value2hsi(z0);
                            out_row[c] = hsi2value(h, s, v);
                        } else {
                            out_row[c] = v;
                        }
                    }
                });

            for (r, row) in rows_buf.iter().enumerate() {
                output
                    .set_row_slice(band, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
        }

        Ok(())
    }

    fn run_fast_almost_gaussian(
        input: &Raster,
        sigma: f64,
        packed_rgb: bool,
    ) -> Result<Raster, ToolError> {
        let n = 5isize;
        let sigma_eff = sigma.max(1.8);
        let w_ideal = (12.0 * sigma_eff * sigma_eff / n as f64 + 1.0).sqrt();
        let mut wl = w_ideal.floor() as isize;
        if wl % 2 == 0 {
            wl -= 1;
        }
        let wu = wl + 2;
        let m = ((12.0 * sigma_eff * sigma_eff
            - (n * wl * wl) as f64
            - (4 * n * wl) as f64
            - (3 * n) as f64)
            / (-4 * wl - 4) as f64)
            .round() as isize;

        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let mut current: Vec<Vec<f64>> = (0..bands)
            .map(|band_idx| {
                let band = band_idx as isize;
                let mut v = vec![nodata; rows * cols];
                for r in 0..rows {
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if !input.is_nodata(z) {
                            v[r * cols + c] = if packed_rgb { value2i(z) } else { z };
                        }
                    }
                }
                v
            })
            .collect();

        for iter in 0..n {
            let width = if iter <= m { wl } else { wu } as usize;
            let radius = (width / 2) as isize;

            current = current
                .into_par_iter()
                .map(|band_vals| {
                    let stride = cols + 1;
                    let mut integral_sum = vec![0.0f64; (rows + 1) * (cols + 1)];
                    let mut integral_count = vec![0u32; (rows + 1) * (cols + 1)];

                    for r in 0..rows {
                        let mut row_sum = 0.0f64;
                        let mut row_count = 0u32;
                        let ir = (r + 1) * stride;
                        let ir_prev = r * stride;
                        for c in 0..cols {
                            let z = band_vals[r * cols + c];
                            if z != nodata {
                                row_sum += z;
                                row_count += 1;
                            }
                            let idx = ir + (c + 1);
                            integral_sum[idx] = integral_sum[ir_prev + (c + 1)] + row_sum;
                            integral_count[idx] = integral_count[ir_prev + (c + 1)] + row_count;
                        }
                    }

                    let mut out = vec![nodata; rows * cols];
                    out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                        for c in 0..cols {
                            let z0 = band_vals[r * cols + c];
                            if z0 == nodata {
                                continue;
                            }

                            let y1 = (r as isize - radius).max(0) as usize;
                            let y2 = (r as isize + radius).min((rows - 1) as isize) as usize;
                            let x1 = (c as isize - radius).max(0) as usize;
                            let x2 = (c as isize + radius).min((cols - 1) as isize) as usize;

                            let a = y1 * stride + x1;
                            let b = y1 * stride + (x2 + 1);
                            let cidx = (y2 + 1) * stride + x1;
                            let d = (y2 + 1) * stride + (x2 + 1);

                            let n = (integral_count[d] + integral_count[a] - integral_count[b]
                                - integral_count[cidx])
                                as f64;
                            if n > 0.0 {
                                let sum = integral_sum[d] + integral_sum[a]
                                    - integral_sum[b]
                                    - integral_sum[cidx];
                                out_row[c] = sum / n;
                            }
                        }
                    });

                    out
                })
                .collect();
        }

        let mut out = input.clone();
        Self::write_values_into_output(input, &mut out, &current, packed_rgb)?;
        Ok(out)
    }

    fn normalize_odd_size(v: usize, minimum: usize) -> usize {
        let mut s = v.max(minimum);
        if s % 2 == 0 {
            s += 1;
        }
        s
    }

    fn parse_window_sizes(args: &ToolArgs, default_x: usize, default_y: usize) -> (usize, usize, isize, isize) {
        let sx = Self::normalize_odd_size(
            args.get("filter_size_x").and_then(|v| v.as_u64()).unwrap_or(default_x as u64) as usize,
            3,
        );
        let sy = Self::normalize_odd_size(
            args.get("filter_size_y").and_then(|v| v.as_u64()).unwrap_or(default_y as u64) as usize,
            3,
        );
        let mx = (sx as f64 / 2.0).floor() as isize;
        let my = (sy as f64 / 2.0).floor() as isize;
        (sx, sy, mx, my)
    }

    fn min_max_by_band(input: &Raster, packed_rgb: bool) -> Vec<(f64, f64)> {
        (0..input.bands)
            .into_par_iter()
            .map(|band_idx| {
                let band = band_idx as isize;
                let mut min_v = f64::INFINITY;
                let mut max_v = f64::NEG_INFINITY;
                for r in 0..input.rows {
                    for c in 0..input.cols {
                        let z_raw = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z_raw) {
                            continue;
                        }
                        let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                        min_v = min_v.min(z);
                        max_v = max_v.max(z);
                    }
                }
                if min_v == f64::INFINITY {
                    (0.0, 0.0)
                } else {
                    (min_v, max_v)
                }
            })
            .collect()
    }

    fn run_with_op(op: Phase3Op, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info(&format!("running {}", op.id()));
        let input = Self::load_raster(&input_path)?;
        let rgb_mode = color_support::detect_rgb_mode(&input, false, true);
        let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && input.bands == 1;

        let output = match op {
            Phase3Op::FastAlmostGaussian => {
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.8)
                    .max(1.8);
                Self::run_fast_almost_gaussian(&input, sigma, packed_rgb)?
            }
            Phase3Op::EdgePreservingMean => {
                let mut filter_size = args
                    .get("filter_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(11) as usize;
                if filter_size < 3 {
                    filter_size = 3;
                }
                if filter_size % 2 == 0 {
                    filter_size += 1;
                }
                let threshold = args
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(15.0);

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let radius = (filter_size / 2) as isize;

                let mut out = input.as_ref().clone();
                let vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut band_buf = vec![nodata; rows * cols];
                        band_buf
                            .par_chunks_mut(cols)
                            .enumerate()
                            .for_each(|(r, row_buf)| {
                                for (c, cell) in row_buf.iter_mut().enumerate() {
                                    let z_raw = input.get(band, r as isize, c as isize);
                                    if input.is_nodata(z_raw) {
                                        continue;
                                    }
                                    *cell = if packed_rgb { value2i(z_raw) } else { z_raw };
                                }
                            });

                        let mut v = vec![nodata; rows * cols];
                        v.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                            let row = r as isize;
                            let row_offset = r * cols;
                            for c in 0..cols {
                                let z0 = band_buf[row_offset + c];
                                if z0 == nodata {
                                    continue;
                                }
                                let mut sum = 0.0;
                                let mut cnt = 0.0;
                                for ny in (row - radius)..=(row + radius) {
                                    if ny < 0 || ny >= rows as isize {
                                        continue;
                                    }
                                    let ny_offset = ny as usize * cols;
                                    for nx in (c as isize - radius)..=(c as isize + radius) {
                                        if nx < 0 || nx >= cols as isize {
                                            continue;
                                        }
                                        let zn = band_buf[ny_offset + nx as usize];
                                        if zn == nodata {
                                            continue;
                                        }
                                        if (zn - z0).abs() <= threshold {
                                            sum += zn;
                                            cnt += 1.0;
                                        }
                                    }
                                }
                                if cnt > 0.0 {
                                    row_out[c] = sum / cnt;
                                }
                            }
                        });
                        v
                    })
                    .collect();

                Self::write_values_into_output(&input, &mut out, &vals, packed_rgb)?;
                out
            }
            Phase3Op::Unsharp => {
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.75)
                    .clamp(0.5, 20.0);
                let amount = args
                    .get("amount")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(100.0);
                let threshold = args
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    .abs();

                let blur_vals = Self::gaussian_blur_values(&input, sigma, packed_rgb);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut v = vec![nodata; rows * cols];
                        v.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                            for c in 0..cols {
                                let idx = r * cols + c;
                                let z_raw = input.get(band, r as isize, c as isize);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let b = blur_vals[band_idx][idx];
                                if b == nodata {
                                    row_out[c] = z;
                                    continue;
                                }
                                let diff = z - b;
                                row_out[c] = if diff.abs() > threshold {
                                    z + diff * amount
                                } else {
                                    z
                                };
                            }
                        });
                        v
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::DiffOfGaussians => {
                let mut sigma1 = args
                    .get("sigma1")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(2.0)
                    .clamp(0.25, 20.0);
                let mut sigma2 = args
                    .get("sigma2")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(4.0)
                    .clamp(0.5, 20.0);
                if (sigma1 - sigma2).abs() < f64::EPSILON {
                    return Err(ToolError::Validation(
                        "sigma1 and sigma2 must not be equal".to_string(),
                    ));
                }
                if sigma1 > sigma2 {
                    std::mem::swap(&mut sigma1, &mut sigma2);
                }

                let g1 = Self::gaussian_blur_values(&input, sigma1, packed_rgb);
                let g2 = Self::gaussian_blur_values(&input, sigma2, packed_rgb);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let mut v = vec![nodata; rows * cols];
                        v.par_chunks_mut(cols).enumerate().for_each(|(r, row_out)| {
                            for c in 0..cols {
                                let idx = r * cols + c;
                                let a = g1[band_idx][idx];
                                let b = g2[band_idx][idx];
                                if a != nodata && b != nodata {
                                    row_out[c] = a - b;
                                }
                            }
                        });
                        v
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::Adaptive => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let threshold = args
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(2.0)
                    .abs();

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let mins = Self::min_max_by_band(&input, packed_rgb)
                    .into_iter()
                    .map(|(min_v, _)| min_v)
                    .collect::<Vec<_>>();

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let min_val = mins[band_idx];
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let mut n = 0.0;
                                let mut sum = 0.0;
                                let mut sum2 = 0.0;
                                for ny in (row - my)..=(row + my) {
                                    for nx in (col - mx)..=(col + mx) {
                                        let zn_raw = input.get(band, ny, nx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        let zr = zn - min_val;
                                        sum += zr;
                                        sum2 += zr * zr;
                                        n += 1.0;
                                    }
                                }
                                if n <= 0.0 {
                                    continue;
                                }
                                let variance = (sum2 - (sum * sum) / n) / n;
                                if variance > 0.0 {
                                    let s = variance.sqrt();
                                    let mean = sum / n + min_val;
                                    out_row[c] = if ((z - mean) / s).abs() > threshold { mean } else { z };
                                } else {
                                    out_row[c] = z;
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::Lee | Phase3Op::EnhancedLee => {
                let (_sx, _sy, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let enhanced = matches!(op, Phase3Op::EnhancedLee);
                let default_enl = if enhanced { 4.0 } else { 1.0 };
                let enl = args
                    .get("enl")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default_enl);
                if !enl.is_finite() || enl <= 0.0 {
                    return Err(ToolError::Validation("enl must be a positive number".to_string()));
                }
                let cu = 1.0 / enl.sqrt();
                let cmax = (1.0 + 2.0 / enl).sqrt();

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };

                                let mut sum = 0.0;
                                let mut sum2 = 0.0;
                                let mut n = 0.0;
                                for ny in (row - my)..=(row + my) {
                                    for nx in (col - mx)..=(col + mx) {
                                        let zn_raw = input.get(band, ny, nx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        // Shift by the centre value so the variance
                                        // does not cancel catastrophically.
                                        let d = zn - z;
                                        sum += d;
                                        sum2 += d * d;
                                        n += 1.0;
                                    }
                                }

                                out_row[c] = if enhanced {
                                    enhanced_lee_estimate(z, sum, sum2, n, cu, cmax)
                                } else {
                                    lee_mmse_estimate(z, sum, sum2, n, cu)
                                };
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::LeeSigma => {
                let (sx, sy, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(10.0)
                    .abs();
                let mut m_value = args
                    .get("m_value")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(5.0);
                let max_cells = (sx * sy) as f64;
                if m_value > max_cells {
                    m_value = max_cells;
                }

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let n8 = [
                    (-1isize, -1isize),
                    (-1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                    (1, 0),
                    (1, -1),
                    (0, -1),
                ];

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let lo = z - sigma;
                                let hi = z + sigma;

                                let mut sum = 0.0;
                                let mut n = 0.0;
                                for ny in (row - my)..=(row + my) {
                                    for nx in (col - mx)..=(col + mx) {
                                        let zn_raw = input.get(band, ny, nx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        if zn >= lo && zn <= hi {
                                            sum += zn;
                                            n += 1.0;
                                        }
                                    }
                                }

                                if n > m_value {
                                    out_row[c] = sum / n;
                                } else {
                                    let mut s2 = 0.0;
                                    let mut n2 = 0.0;
                                    for (dy, dx) in &n8 {
                                        let zn_raw = input.get(band, row + *dy, col + *dx);
                                        if input.is_nodata(zn_raw) {
                                            continue;
                                        }
                                        let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                        s2 += zn;
                                        n2 += 1.0;
                                    }
                                    if n2 > 0.0 {
                                        out_row[c] = s2 / n2;
                                    }
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::RefinedLee => {
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };

                                // Scan 3x3 sub-windows within the filter kernel
                                let mut min_cov = f64::INFINITY;
                                let mut best_window_sum = 0.0;
                                let mut best_window_count = 0.0;

                                // The nine 3x3 sub-windows of the 5x5 neighbourhood
                                // that contain the centre pixel.
                                let window_offsets = [
                                    (-1, -1), (-1, 0), (-1, 1),
                                    (0, -1), (0, 0), (0, 1),
                                    (1, -1), (1, 0), (1, 1),
                                ];

                                for (wy, wx) in &window_offsets {
                                    let mut sum = 0.0;
                                    let mut sum2 = 0.0;
                                    let mut n = 0.0;
                                    for dy in -1..=1 {
                                        for dx in -1..=1 {
                                            let ny = row + *wy + dy;
                                            let nx = col + *wx + dx;
                                            let zn_raw = input.get(band, ny, nx);
                                            if input.is_nodata(zn_raw) {
                                                continue;
                                            }
                                            let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                            sum += zn;
                                            sum2 += zn * zn;
                                            n += 1.0;
                                        }
                                    }
                                    if n > 0.0 {
                                        let mean = sum / n;
                                        let variance = (sum2 - (sum * sum) / n) / n.max(1.0);
                                        let cov = if mean.abs() > f64::EPSILON {
                                            (variance.sqrt()) / mean.abs()
                                        } else {
                                            f64::INFINITY
                                        };
                                        if cov < min_cov {
                                            min_cov = cov;
                                            best_window_sum = sum;
                                            best_window_count = n;
                                        }
                                    }
                                }

                                if best_window_count > 0.0 {
                                    out_row[c] = best_window_sum / best_window_count;
                                } else {
                                    out_row[c] = z;
                                }
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::ConservativeSmoothing => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 3, 3);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let filter_width = 2 * mx as usize + 1;
                        let use_staged = filter_width > 3;
                        let band_buf = if use_staged {
                            let mut buf = vec![nodata; rows * cols];
                            buf.par_chunks_mut(cols)
                                .enumerate()
                                .for_each(|(r, row_buf)| {
                                    for (c, cell) in row_buf.iter_mut().enumerate() {
                                        let z_raw = input.get(band, r as isize, c as isize);
                                        if input.is_nodata(z_raw) {
                                            continue;
                                        }
                                        *cell = if packed_rgb { value2i(z_raw) } else { z_raw };
                                    }
                                });
                            Some(buf)
                        } else {
                            None
                        };

                        let fetch_value = |rr: usize, cc: usize, band_buf: &Option<Vec<f64>>| -> f64 {
                            if let Some(buf) = band_buf {
                                return buf[rr * cols + cc];
                            }
                            let z_raw = input.get(band, rr as isize, cc as isize);
                            if input.is_nodata(z_raw) {
                                nodata
                            } else if packed_rgb {
                                value2i(z_raw)
                            } else {
                                z_raw
                            }
                        };

                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            let start_row = (row - my).max(0) as usize;
                            let end_row = (row + my).min(rows as isize - 1) as usize;
                            let mut filter_min_vals = vec![f64::INFINITY; filter_width];
                            let mut filter_max_vals = vec![f64::NEG_INFINITY; filter_width];
                            let mut head = 0usize;

                            for c in 0..cols {
                                let z = fetch_value(r, c, &band_buf);
                                if z == nodata {
                                    continue;
                                }

                                if c > 0 {
                                    let mut col_min = f64::INFINITY;
                                    let mut col_max = f64::NEG_INFINITY;
                                    let new_col = c as isize + mx;
                                    if new_col >= 0 && new_col < cols as isize {
                                        let new_col = new_col as usize;
                                        for rr in start_row..=end_row {
                                            let zn = fetch_value(rr, new_col, &band_buf);
                                            if zn == nodata {
                                                continue;
                                            }
                                            if zn < col_min {
                                                col_min = zn;
                                            }
                                            if zn > col_max {
                                                col_max = zn;
                                            }
                                        }
                                    }
                                    filter_min_vals[head] = col_min;
                                    filter_max_vals[head] = col_max;
                                    head = (head + 1) % filter_width;
                                } else {
                                    for i in 0..filter_width {
                                        let cc = i as isize - mx;
                                        let mut col_min = f64::INFINITY;
                                        let mut col_max = f64::NEG_INFINITY;
                                        if cc >= 0 && cc < cols as isize {
                                            let cc = cc as usize;
                                            for rr in start_row..=end_row {
                                                let zn = fetch_value(rr, cc, &band_buf);
                                                if zn == nodata {
                                                    continue;
                                                }
                                                if zn < col_min {
                                                    col_min = zn;
                                                }
                                                if zn > col_max {
                                                    col_max = zn;
                                                }
                                            }
                                        }
                                        filter_min_vals[i] = col_min;
                                        filter_max_vals[i] = col_max;
                                    }
                                }

                                let mut min_v = f64::INFINITY;
                                let mut max_v = f64::NEG_INFINITY;
                                let mut min2_v = f64::INFINITY;
                                let mut max2_v = f64::NEG_INFINITY;
                                for i in 0..filter_width {
                                    let col_min = filter_min_vals[i];
                                    let col_max = filter_max_vals[i];
                                    if col_min < min_v {
                                        min2_v = min_v;
                                        min_v = col_min;
                                    } else if col_min < min2_v {
                                        min2_v = col_min;
                                    }
                                    if col_max > max_v {
                                        max2_v = max_v;
                                        max_v = col_max;
                                    } else if col_max > max2_v {
                                        max2_v = col_max;
                                    }
                                }

                                out_row[c] = if z > min_v && z < max_v {
                                    z
                                } else if z == min_v {
                                    if min2_v.is_finite() { min2_v } else { min_v }
                                } else if z == max_v {
                                    if max2_v.is_finite() { max2_v } else { max_v }
                                } else {
                                    z
                                };
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::Olympic => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let filter_width = 2 * mx as usize + 1;
                        let mut band_buf = vec![nodata; rows * cols];
                        band_buf
                            .par_chunks_mut(cols)
                            .enumerate()
                            .for_each(|(r, row_buf)| {
                                for (c, cell) in row_buf.iter_mut().enumerate() {
                                    let z_raw = input.get(band, r as isize, c as isize);
                                    if input.is_nodata(z_raw) {
                                        continue;
                                    }
                                    *cell = if packed_rgb { value2i(z_raw) } else { z_raw };
                                }
                            });

                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let start_row = (r as isize - my).max(0) as usize;
                            let end_row = (r as isize + my).min(rows as isize - 1) as usize;
                            let mut filter_min_vals = vec![f64::INFINITY; filter_width];
                            let mut filter_max_vals = vec![f64::NEG_INFINITY; filter_width];
                            let mut filter_totals = vec![0.0; filter_width];
                            let mut filter_counts = vec![0usize; filter_width];
                            let mut head = 0usize;

                            for c in 0..cols {
                                let z = band_buf[r * cols + c];
                                if z == nodata {
                                    continue;
                                }

                                if c > 0 {
                                    let mut col_min = f64::INFINITY;
                                    let mut col_max = f64::NEG_INFINITY;
                                    let mut col_total = 0.0;
                                    let mut col_count = 0usize;
                                    let new_col = c as isize + mx;
                                    if new_col >= 0 && new_col < cols as isize {
                                        let new_col = new_col as usize;
                                        for rr in start_row..=end_row {
                                            let zn = band_buf[rr * cols + new_col];
                                            if zn == nodata {
                                                continue;
                                            }
                                            if zn < col_min {
                                                col_min = zn;
                                            }
                                            if zn > col_max {
                                                col_max = zn;
                                            }
                                            col_total += zn;
                                            col_count += 1;
                                        }
                                    }
                                    filter_min_vals[head] = col_min;
                                    filter_max_vals[head] = col_max;
                                    filter_totals[head] = col_total;
                                    filter_counts[head] = col_count;
                                    head = (head + 1) % filter_width;
                                } else {
                                    for i in 0..filter_width {
                                        let cc = i as isize - mx;
                                        let mut col_min = f64::INFINITY;
                                        let mut col_max = f64::NEG_INFINITY;
                                        let mut col_total = 0.0;
                                        let mut col_count = 0usize;
                                        if cc >= 0 && cc < cols as isize {
                                            let cc = cc as usize;
                                            for rr in start_row..=end_row {
                                                let zn = band_buf[rr * cols + cc];
                                                if zn == nodata {
                                                    continue;
                                                }
                                                if zn < col_min {
                                                    col_min = zn;
                                                }
                                                if zn > col_max {
                                                    col_max = zn;
                                                }
                                                col_total += zn;
                                                col_count += 1;
                                            }
                                        }
                                        filter_min_vals[i] = col_min;
                                        filter_max_vals[i] = col_max;
                                        filter_totals[i] = col_total;
                                        filter_counts[i] = col_count;
                                    }
                                }

                                let mut min_v = f64::INFINITY;
                                let mut max_v = f64::NEG_INFINITY;
                                let mut sum = 0.0;
                                let mut n = 0usize;
                                for i in 0..filter_width {
                                    let col_min = filter_min_vals[i];
                                    let col_max = filter_max_vals[i];
                                    if col_min < min_v {
                                        min_v = col_min;
                                    }
                                    if col_max > max_v {
                                        max_v = col_max;
                                    }
                                    sum += filter_totals[i];
                                    n += filter_counts[i];
                                }

                                out_row[c] = if n > 2 {
                                    (sum - min_v - max_v) / (n - 2) as f64
                                } else {
                                    sum / n as f64
                                };
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::KNearestMean => {
                let (sx, sy, mx, my) = Self::parse_window_sizes(args, 3, 3);
                let mut k = args.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                k += 1;
                let max_cells = sx * sy;
                if k > max_cells {
                    k = max_cells;
                }

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;

                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            let mut best_vals = vec![0.0; k];
                            let mut best_dists = vec![0.0; k];
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let mut best_len = 0usize;

                                if packed_rgb {
                                    for ny in (row - my)..=(row + my) {
                                        for nx in (col - mx)..=(col + mx) {
                                            let zn_raw = input.get(band, ny, nx);
                                            if input.is_nodata(zn_raw) {
                                                continue;
                                            }
                                            let zn = value2i(zn_raw);
                                            let diff2 = (zn - z) * (zn - z);
                                            if best_len < k {
                                                let mut insert_at = best_len;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                                best_len += 1;
                                            } else if diff2 < best_dists[k - 1] {
                                                let mut insert_at = k - 1;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                            }
                                        }
                                    }
                                } else {
                                    for ny in (row - my)..=(row + my) {
                                        for nx in (col - mx)..=(col + mx) {
                                            let zn = input.get(band, ny, nx);
                                            if input.is_nodata(zn) {
                                                continue;
                                            }
                                            let diff2 = (zn - z) * (zn - z);
                                            if best_len < k {
                                                let mut insert_at = best_len;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                                best_len += 1;
                                            } else if diff2 < best_dists[k - 1] {
                                                let mut insert_at = k - 1;
                                                while insert_at > 0 && diff2 < best_dists[insert_at - 1] {
                                                    best_dists[insert_at] = best_dists[insert_at - 1];
                                                    best_vals[insert_at] = best_vals[insert_at - 1];
                                                    insert_at -= 1;
                                                }
                                                best_dists[insert_at] = diff2;
                                                best_vals[insert_at] = zn;
                                            }
                                        }
                                    }
                                }

                                if best_len == 0 {
                                    continue;
                                }

                                let sum = best_vals[..best_len].iter().sum::<f64>();
                                out_row[c] = sum / best_len as f64;
                            }
                        });
                        out
                    })
                    .collect();

                let mut out = input.as_ref().clone();
                Self::write_values_into_output(&input, &mut out, &out_vals, packed_rgb)?;
                out
            }
            Phase3Op::HighPassMedian => {
                let (_, _, mx, my) = Self::parse_window_sizes(args, 11, 11);
                let mut sig_digits = args
                    .get("sig_digits")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as i32;
                if packed_rgb && sig_digits < 4 {
                    sig_digits = 4;
                }
                let multiplier = 10f64.powi(sig_digits);

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let minmax = Self::min_max_by_band(&input, packed_rgb);

                let out_vals: Result<Vec<Vec<f64>>, ToolError> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| -> Result<Vec<f64>, ToolError> {
                        let band = band_idx as isize;
                        let (band_min, band_max) = if packed_rgb {
                            (0.0, 1.0)
                        } else {
                            minmax[band_idx]
                        };
                        if !band_min.is_finite() || !band_max.is_finite() {
                            return Ok(vec![nodata; rows * cols]);
                        }

                        let min_bin = (band_min * multiplier).floor() as i64;
                        let max_bin = (band_max * multiplier).floor() as i64;
                        let num_bins_i64 = (max_bin - min_bin + 1).max(1);
                        let num_bins = usize::try_from(num_bins_i64).map_err(|_| {
                            ToolError::Execution(
                                "high-pass-median histogram bin count exceeds platform limits".to_string(),
                            )
                        })?;

                        let bin_nodata = i64::MIN;
                        let mut binned = vec![bin_nodata; rows * cols];
                        binned
                            .par_chunks_mut(cols)
                            .enumerate()
                            .for_each(|(r, row_bins)| {
                                for (c, cell_bin) in row_bins.iter_mut().enumerate() {
                                    let z_raw = input.get(band, r as isize, c as isize);
                                    if input.is_nodata(z_raw) {
                                        continue;
                                    }
                                    let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                    *cell_bin = (z * multiplier).floor() as i64 - min_bin;
                                }
                            });

                        let rows_isize = rows as isize;
                        let cols_isize = cols as isize;
                        let get_bin = |rr: isize, cc: isize| -> i64 {
                            if rr < 0 || rr >= rows_isize || cc < 0 || cc >= cols_isize {
                                return bin_nodata;
                            }
                            binned[rr as usize * cols + cc as usize]
                        };

                        let mut out = vec![nodata; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            let start_row = row - my;
                            let end_row = row + my;
                            let mut histo = vec![0i64; num_bins];
                            let mut old_median = bin_nodata;
                            let mut median = bin_nodata;
                            let mut n = 0i64;
                            let mut n_less = 0i64;

                            for c in 0..cols {
                                let col = c as isize;
                                let center_bin = get_bin(row, col);
                                if center_bin == bin_nodata {
                                    old_median = bin_nodata;
                                    continue;
                                }

                                if old_median != bin_nodata {
                                    let trailing_col = col - mx - 1;
                                    let leading_col = col + mx;

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, trailing_col);
                                        if bv != bin_nodata {
                                            histo[bv as usize] -= 1;
                                            n -= 1;
                                            if bv < old_median {
                                                n_less -= 1;
                                            }
                                        }
                                    }

                                    for rr in start_row..=end_row {
                                        let bv = get_bin(rr, leading_col);
                                        if bv != bin_nodata {
                                            histo[bv as usize] += 1;
                                            n += 1;
                                            if bv < old_median {
                                                n_less += 1;
                                            }
                                        }
                                    }

                                    let target = n / 2;
                                    if n_less < target {
                                        let mut v = old_median;
                                        while v < num_bins_i64 {
                                            let hv = histo[v as usize];
                                            if n_less + hv >= target {
                                                median = v;
                                                break;
                                            }
                                            n_less += hv;
                                            v += 1;
                                        }
                                    } else {
                                        let mut v = old_median - 1;
                                        while v >= 0 {
                                            let hv = histo[v as usize];
                                            if n_less - hv >= target {
                                                n_less -= hv;
                                                v -= 1;
                                            } else {
                                                median = v + 1;
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    histo.fill(0);
                                    n = 0;
                                    n_less = 0;
                                    let start_col = col - mx;
                                    let end_col = col + mx;

                                    for cc in start_col..=end_col {
                                        for rr in start_row..=end_row {
                                            let bv = get_bin(rr, cc);
                                            if bv != bin_nodata {
                                                histo[bv as usize] += 1;
                                                n += 1;
                                            }
                                        }
                                    }

                                    let target = n / 2;
                                    let mut acc = 0i64;
                                    for (i, hv) in histo.iter().enumerate() {
                                        acc += *hv;
                                        if acc >= target {
                                            median = i as i64;
                                            break;
                                        }
                                        n_less = acc;
                                    }
                                }

                                if n > 0 {
                                    out_row[c] = (center_bin - median) as f64 / multiplier;
                                }
                                old_median = median;
                            }
                        });

                        Ok(out)
                    })
                    .collect();
                let out_vals = out_vals?;

                if packed_rgb {
                    let rows = input.rows;
                    let cols = input.cols;
                    let mut out = input.as_ref().clone();
                    for band_idx in 0..input.bands {
                        let band = band_idx as isize;
                        let mut rows_buf = vec![vec![nodata; cols]; rows];
                        rows_buf
                            .par_iter_mut()
                            .enumerate()
                            .for_each(|(r, out_row)| {
                                for c in 0..cols {
                                    let idx = r * cols + c;
                                    let v = out_vals[band_idx][idx];
                                    if v == nodata {
                                        continue;
                                    }
                                    let z0 = input.get(band, r as isize, c as isize);
                                    let (h, s, _) = value2hsi(z0);
                                    out_row[c] = hsi2value(h, s, v + 0.5);
                                }
                            });
                        for (r, row) in rows_buf.iter().enumerate() {
                            out
                                .set_row_slice(band, r as isize, row)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing row {}: {}",
                                        r, e
                                    ))
                                })?;
                        }
                    }
                    out
                } else {
                    let mut out = input.as_ref().clone();
                    Self::write_values_into_output(&input, &mut out, &out_vals, false)?;
                    out
                }
            }
            Phase3Op::LaplacianOfGaussians => {
                let sigma = args
                    .get("sigma")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.75)
                    .clamp(0.5, 20.0);

                let recip = 1.0 / ((2.0 * PI).sqrt() * sigma);
                let two_sigma_sqr = 2.0 * sigma * sigma;
                let mut filter_size = 0usize;
                for i in 0..250usize {
                    let weight = recip * (-((i * i) as f64) / two_sigma_sqr).exp();
                    if weight <= 0.001 {
                        filter_size = i * 2 + 1;
                        break;
                    }
                }
                filter_size = Self::normalize_odd_size(filter_size, 3);
                let radius = (filter_size as f64 / 2.0).floor() as isize;

                let term1 = -1.0 / (PI * sigma.powi(4));
                let mut kernel = Vec::with_capacity(filter_size * filter_size);
                for ry in 0..filter_size {
                    for rx in 0..filter_size {
                        let x = rx as isize - radius;
                        let y = ry as isize - radius;
                        let dist2 = (x * x + y * y) as f64;
                        let term2 = 1.0 - dist2 / two_sigma_sqr;
                        let term3 = (-dist2 / two_sigma_sqr).exp();
                        kernel.push((y, x, term1 * term2 * term3));
                    }
                }

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let out_vals: Vec<Vec<f64>> = (0..bands)
                    .into_par_iter()
                    .map(|band_idx| {
                        let band = band_idx as isize;
                        // NaN marks NoData cells; any finite value is a real response.
                        let mut out = vec![f64::NAN; rows * cols];
                        out.par_chunks_mut(cols).enumerate().for_each(|(r, out_row)| {
                            let row = r as isize;
                            for c in 0..cols {
                                let col = c as isize;
                                let z_raw = input.get(band, row, col);
                                if input.is_nodata(z_raw) {
                                    continue;
                                }
                                // Convolve the differences from the centre value. This
                                // is the LoG response with the truncated kernel made
                                // zero-sum over the valid cells, so flat areas give 0
                                // even next to NoData. Dividing by the kernel's sum, as
                                // before, returned roughly the input value instead.
                                let z = if packed_rgb { value2i(z_raw) } else { z_raw };
                                let mut response = 0.0;
                                for &(dy, dx, w) in &kernel {
                                    let zn_raw = input.get(band, row + dy, col + dx);
                                    if input.is_nodata(zn_raw) {
                                        continue;
                                    }
                                    let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                    response += w * (zn - z);
                                }
                                out_row[c] = response;
                            }
                        });
                        out
                    })
                    .collect();

                // The response is signed and near zero, so write it to a new
                // Float32 raster: the input's integer type would truncate it, and
                // its NoData (often 0) would swallow flat areas. Packed RGB input
                // is filtered on intensity and written as a scalar.
                let out_nodata = -32768.0;
                let mut out = Raster::new(RasterConfig {
                    cols,
                    rows,
                    bands,
                    x_min: input.x_min,
                    y_min: input.y_min,
                    cell_size: input.cell_size_x,
                    cell_size_y: Some(input.cell_size_y),
                    nodata: out_nodata,
                    data_type: DataType::F32,
                    crs: input.crs.clone(),
                    metadata: input.metadata.clone(),
                });
                for (band_idx, vals) in out_vals.iter().enumerate() {
                    for r in 0..rows {
                        for c in 0..cols {
                            let v = vals[r * cols + c];
                            let v = if v.is_nan() { out_nodata } else { v };
                            out.set(band_idx as isize, r as isize, c as isize, v).map_err(|e| {
                                ToolError::Execution(format!("failed writing output raster: {e}"))
                            })?;
                        }
                    }
                }
                out
            }
        };

        ctx.progress.progress(1.0);
        let output_locator = Self::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

/// Window mean and coefficient of variation from shifted running sums.
///
/// Args:
///     z: Centre pixel value, the shift applied to the sums.
///     sum: Sum of `(value - z)` over the valid window values.
///     sum2: Sum of `(value - z)^2`.
///     n: Number of valid window values.
///
/// Returns:
///     `Some((mean, ci))` when the window has at least two values and a
///     positive mean, where `ci = std / mean`; `None` otherwise.
fn window_mean_and_cv(z: f64, sum: f64, sum2: f64, n: f64) -> Option<(f64, f64)> {
    if n < 2.0 {
        return None;
    }
    let mean = z + sum / n;
    if !(mean > 0.0) {
        return None;
    }
    let variance = ((sum2 - sum * sum / n) / n).max(0.0);
    Some((mean, variance.sqrt() / mean))
}

/// Lee MMSE speckle estimate for one pixel.
///
/// Args:
///     z: Centre pixel value (linear backscatter).
///     sum: Sum of `(value - z)` over the window.
///     sum2: Sum of `(value - z)^2` over the window.
///     n: Number of valid window values.
///     cu: Speckle coefficient of variation, `1 / sqrt(ENL)`.
///
/// Returns:
///     `mean + w * (z - mean)` with `w = clamp(1 - cu^2 / ci^2, 0, 1)`; `z`
///     unchanged when the window has no positive mean.
fn lee_mmse_estimate(z: f64, sum: f64, sum2: f64, n: f64, cu: f64) -> f64 {
    let Some((mean, ci)) = window_mean_and_cv(z, sum, sum2, n) else {
        return z;
    };
    let w = if ci > 0.0 {
        (1.0 - (cu * cu) / (ci * ci)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    mean + w * (z - mean)
}

/// Enhanced Lee (Lopes et al., 1990) speckle estimate for one pixel.
///
/// Args:
///     z: Centre pixel value (linear backscatter).
///     sum: Sum of `(value - z)` over the window.
///     sum2: Sum of `(value - z)^2` over the window.
///     n: Number of valid window values.
///     cu: Speckle coefficient of variation, `1 / sqrt(ENL)`.
///     cmax: Upper heterogeneity limit, `sqrt(1 + 2 / ENL)`.
///
/// Returns:
///     The window mean when `ci <= cu`, `z` when `ci >= cmax`, and an
///     exponentially damped blend in between; `z` unchanged when the window
///     has no positive mean.
fn enhanced_lee_estimate(z: f64, sum: f64, sum2: f64, n: f64, cu: f64, cmax: f64) -> f64 {
    let Some((mean, ci)) = window_mean_and_cv(z, sum, sum2, n) else {
        return z;
    };
    if ci <= cu {
        mean
    } else if ci >= cmax {
        z
    } else {
        let w = (-(ci - cu) / (cmax - ci)).exp();
        mean * w + z * (1.0 - w)
    }
}

macro_rules! define_phase3_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                FastAlmostGaussianFilterTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                FastAlmostGaussianFilterTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = FastAlmostGaussianFilterTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                FastAlmostGaussianFilterTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_phase3_tool!(FastAlmostGaussianFilterTool, Phase3Op::FastAlmostGaussian);
define_phase3_tool!(EdgePreservingMeanFilterTool, Phase3Op::EdgePreservingMean);
define_phase3_tool!(UnsharpMaskingTool, Phase3Op::Unsharp);
define_phase3_tool!(DiffOfGaussiansFilterTool, Phase3Op::DiffOfGaussians);
define_phase3_tool!(AdaptiveFilterTool, Phase3Op::Adaptive);
define_phase3_tool!(LeeFilterTool, Phase3Op::Lee);
define_phase3_tool!(LeeSigmaFilterTool, Phase3Op::LeeSigma);
define_phase3_tool!(RefinedLeeFilterTool, Phase3Op::RefinedLee);
define_phase3_tool!(EnhancedLeeFilterTool, Phase3Op::EnhancedLee);
define_phase3_tool!(ConservativeSmoothingFilterTool, Phase3Op::ConservativeSmoothing);
define_phase3_tool!(OlympicFilterTool, Phase3Op::Olympic);
define_phase3_tool!(KNearestMeanFilterTool, Phase3Op::KNearestMean);
define_phase3_tool!(HighPassMedianFilterTool, Phase3Op::HighPassMedian);
define_phase3_tool!(LaplacianOfGaussiansFilterTool, Phase3Op::LaplacianOfGaussians);

#[cfg(test)]
mod tests {
    use super::*;
    use wbcore::{AllowAllCapabilities, ProgressSink, ToolContext};
    use wbraster::RasterConfig;

    struct NoopProgress;
    impl ProgressSink for NoopProgress {}

    fn make_ctx() -> ToolContext<'static> {
        static PROGRESS: NoopProgress = NoopProgress;
        static CAPS: AllowAllCapabilities = AllowAllCapabilities;
        ToolContext {
            progress: &PROGRESS,
            capabilities: &CAPS,
        }
    }

    fn make_constant_raster(rows: usize, cols: usize, value: f64) -> Raster {
        let cfg = RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                r.set(0, row, col, value).unwrap();
            }
        }
        r
    }

    fn run_with_memory(tool: &dyn Tool, args: &mut ToolArgs, input: Raster) -> Raster {
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);
        args.insert("input".to_string(), json!(input_path));
        let result = tool.run(args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    #[test]
    fn fast_almost_gaussian_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(2.0));
        let out = run_with_memory(
            &FastAlmostGaussianFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn edge_preserving_mean_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size".to_string(), json!(7));
        args.insert("threshold".to_string(), json!(1.0));
        let out = run_with_memory(
            &EdgePreservingMeanFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn unsharp_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(1.0));
        args.insert("amount".to_string(), json!(100.0));
        args.insert("threshold".to_string(), json!(0.0));
        let out = run_with_memory(&UnsharpMaskingTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn dog_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("sigma1".to_string(), json!(2.0));
        args.insert("sigma2".to_string(), json!(4.0));
        let out = run_with_memory(
            &DiffOfGaussiansFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }

    #[test]
    fn adaptive_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("threshold".to_string(), json!(2.0));
        let out = run_with_memory(&AdaptiveFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn lee_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("enl".to_string(), json!(4.0));
        let out = run_with_memory(&LeeFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn lee_sigma_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("sigma".to_string(), json!(10.0));
        args.insert("m_value".to_string(), json!(5.0));
        let out = run_with_memory(&LeeSigmaFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    /// 100 x 100 linear-power image with 4-look gamma speckle: mean 0.02 in
    /// columns 0-49 and 0.2 in columns 50-99, as in whitebox-wasm#66.
    fn make_two_region_speckle_raster(scale: f64) -> Raster {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut uniform = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ((state >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        let mut r = make_constant_raster(100, 100, 0.0);
        for row in 0..100isize {
            for col in 0..100isize {
                let mean = if col < 50 { 0.02 } else { 0.2 };
                // Gamma(4, 1/4) as the mean of four unit exponentials.
                let speckle = (0..4).map(|_| -uniform().ln()).sum::<f64>() / 4.0;
                r.set(0, row, col, scale * mean * speckle).unwrap();
            }
        }
        r
    }

    fn db(v: f64) -> f64 {
        10.0 * v.log10()
    }

    /// Standard deviation of dB values over rows 5-94 and columns `cols`.
    fn db_std(r: &Raster, cols: std::ops::Range<isize>) -> f64 {
        let vals: Vec<f64> = (5..95isize)
            .flat_map(|row| cols.clone().map(move |col| (row, col)))
            .map(|(row, col)| db(r.get(0, row, col)))
            .collect();
        let m = vals.iter().sum::<f64>() / vals.len() as f64;
        (vals.iter().map(|v| (v - m).powi(2)).sum::<f64>() / vals.len() as f64).sqrt()
    }

    /// dB of the mean linear value of column `col` over rows 5-94.
    fn column_db(r: &Raster, col: isize) -> f64 {
        db((5..95isize).map(|row| r.get(0, row, col)).sum::<f64>() / 90.0)
    }

    fn run_speckle_filter(tool: &dyn Tool, enl: f64, scale: f64) -> Raster {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(5));
        args.insert("filter_size_y".to_string(), json!(5));
        args.insert("enl".to_string(), json!(enl));
        run_with_memory(tool, &mut args, make_two_region_speckle_raster(scale))
    }

    fn assert_reduces_speckle_and_keeps_edge(tool: &dyn Tool, name: &str) {
        let input = make_two_region_speckle_raster(1.0);
        let out = run_speckle_filter(tool, 4.0, 1.0);

        // Speckle drops in both uniform halves.
        for cols in [5..44isize, 56..95isize] {
            let before = db_std(&input, cols.clone());
            let after = db_std(&out, cols.clone());
            assert!(after < 0.5 * before, "{name}: dB std over {cols:?} {before:.2} -> {after:.2}");
        }

        // The edge between columns 49 and 50 stays within one pixel: two
        // columns away on either side the level is within 1.5 dB of the truth
        // (-17 dB and -7 dB). A 5 x 5 mean gives about -12.6 dB and -7.7 dB
        // there, the blur reported in the issue.
        let left = column_db(&out, 48);
        let right = column_db(&out, 51);
        assert!((left - db(0.02)).abs() < 1.5, "{name}: column 48 at {left:.2} dB");
        assert!((right - db(0.2)).abs() < 1.5, "{name}: column 51 at {right:.2} dB");

        // Scale-invariant: multiplying the input by 1000 multiplies the
        // output by 1000.
        let scaled = run_speckle_filter(tool, 4.0, 1000.0);
        for row in 0..100isize {
            for col in 0..100isize {
                let a = out.get(0, row, col);
                let b = scaled.get(0, row, col) / 1000.0;
                assert!((a - b).abs() <= 1e-4 * a.abs(), "{name}: ({row}, {col}) {a} vs {b}");
            }
        }
    }

    #[test]
    fn lee_filter_reduces_speckle_keeps_edges_and_is_scale_invariant() {
        assert_reduces_speckle_and_keeps_edge(&LeeFilterTool, "lee_filter");
    }

    #[test]
    fn enhanced_lee_filter_reduces_speckle_keeps_edges_and_is_scale_invariant() {
        assert_reduces_speckle_and_keeps_edge(&EnhancedLeeFilterTool, "enhanced_lee_filter");
    }

    #[test]
    fn lee_filter_is_not_a_box_mean_on_linear_backscatter() {
        // The old lee_filter averaged every neighbour within +/-10 of the centre,
        // so on linear backscatter (< 1) it returned the plain window mean.
        let out = run_speckle_filter(&LeeFilterTool, 4.0, 1.0);
        let input = make_two_region_speckle_raster(1.0);
        let window_mean = (47..=51isize)
            .flat_map(|c| (8..=12isize).map(move |r| (r, c)))
            .map(|(r, c)| input.get(0, r, c))
            .sum::<f64>()
            / 25.0;
        assert!((out.get(0, 10, 49) - window_mean).abs() > 1e-3);
    }

    #[test]
    fn refined_lee_treats_all_directions_alike() {
        // It used to skip the sub-window to the lower right, so rotating the
        // image by 180 degrees changed the result.
        let input = make_two_region_speckle_raster(1.0);
        let mut rotated = make_constant_raster(100, 100, 0.0);
        for r in 0..100isize {
            for c in 0..100isize {
                rotated.set(0, 99 - r, 99 - c, input.get(0, r, c)).unwrap();
            }
        }
        let out = run_with_memory(&RefinedLeeFilterTool, &mut ToolArgs::new(), input);
        let out_rot = run_with_memory(&RefinedLeeFilterTool, &mut ToolArgs::new(), rotated);
        for r in 2..98isize {
            for c in 2..98isize {
                let a = out.get(0, r, c);
                let b = out_rot.get(0, 99 - r, 99 - c);
                assert!((a - b).abs() <= 1e-6 * a.abs() + 1e-12, "({r}, {c}): {a} vs {b}");
            }
        }
    }

    #[test]
    fn log_output_is_float_and_keeps_flat_areas_when_nodata_is_zero() {
        let mut input = Raster::new(RasterConfig {
            rows: 25,
            cols: 25,
            bands: 1,
            nodata: 0.0,
            data_type: DataType::U8,
            ..Default::default()
        });
        for r in 0..25isize {
            for c in 0..25isize {
                input.set(0, r, c, if c < 13 { 10.0 } else { 20.0 }).unwrap();
            }
        }
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(1.0));
        let out = run_with_memory(&LaplacianOfGaussiansFilterTool, &mut args, input);
        assert_eq!(out.data_type, DataType::F32);
        assert!(!out.is_nodata(out.get(0, 12, 3)), "flat cell became NoData");
        assert_eq!(out.get(0, 12, 3), 0.0);
        assert!(out.get(0, 12, 12) * out.get(0, 12, 13) < 0.0);
    }

    #[test]
    fn log_responds_to_a_step_with_a_zero_crossing() {
        let mut step = make_constant_raster(25, 25, 10.0);
        for r in 0..25isize {
            for c in 13..25isize {
                step.set(0, r, c, 20.0).unwrap();
            }
        }
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(1.0));
        let out = run_with_memory(&LaplacianOfGaussiansFilterTool, &mut args, step);
        let (dark, bright) = (out.get(0, 12, 12), out.get(0, 12, 13));
        // Opposite signs on either side of the edge, zero far from it.
        assert!(dark * bright < 0.0, "{dark} {bright}");
        assert!(out.get(0, 12, 3).abs() < 1e-9 && out.get(0, 12, 22).abs() < 1e-9);
    }

    #[test]
    fn refined_lee_has_no_window_size_parameters() {
        let meta = RefinedLeeFilterTool.metadata();
        assert!(meta.params.iter().all(|p| !p.name.starts_with("filter_size")));
        assert!(!RefinedLeeFilterTool.manifest().defaults.contains_key("filter_size_x"));
    }

    #[test]
    fn lee_filter_rejects_non_positive_enl() {
        let id = memory_store::put_raster(make_constant_raster(5, 5, 1.0));
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("enl".to_string(), json!(0.0));
        assert!(LeeFilterTool.run(&args, &make_ctx()).is_err());
    }

    #[test]
    fn conservative_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        let out = run_with_memory(
            &ConservativeSmoothingFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn olympic_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        let out = run_with_memory(&OlympicFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn knearest_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        args.insert("k".to_string(), json!(5));
        let out = run_with_memory(&KNearestMeanFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!((out.get(0, 12, 12) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn high_pass_median_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(11));
        args.insert("filter_size_y".to_string(), json!(11));
        args.insert("sig_digits".to_string(), json!(2));
        let out = run_with_memory(&HighPassMedianFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }

    #[test]
    fn log_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("sigma".to_string(), json!(0.75));
        let out = run_with_memory(
            &LaplacianOfGaussiansFilterTool,
            &mut args,
            make_constant_raster(25, 25, 10.0),
        );
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }
}
