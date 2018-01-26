dist_base <- 100
comp_dist <- function(x) log2(dist_base - x)
plot(comp_dist, xlim=c(0, 100))

stair <- function(x) { tmp <- log2(x); 1.0 - tmp / (-5.0)}
plot(stair, xlim=c(0, 1.0))