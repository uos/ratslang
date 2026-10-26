# How each new scan is aligned to the map. This covers the voxel grid the map is
# stored in and the iterated extended Kalman filter update that matches a scan
# against it.
registration {
  map {

    # @recommended
    # Edge length of one voxel, which is also the size of the smallest feature
    # registration can resolve. This is the option with the largest effect on
    # both accuracy and runtime, so change it before anything else.
    resolution = 50cm

    # Single | Multi
    # Single: one fixed-resolution map.
    # Multi: scales of 0.5x, 1x, 2x and 4x `resolution`.
    resolution.type = Multi

    # @recommended
    # How many voxels of the resolution above the map holds at any time. Once it
    # is full, older voxels are dropped, so memory stays bounded on a long run.
    # With CUDA enabled and Single resolution this is rounded up to the next
    # power of two. The multiresolution map keeps the exact total instead.
    capacity = 262144
   
    # How strongly a new scan moves the distribution stored in a voxel. Higher
    # values track changes in the environment faster and keep less history.
    distribution_update_sensitivity = 10.0
  }

  # Maximum filter correction iterations per scan.
  max_iter = 4

  # Largest distance at which a map voxel is still considered a neighbor of a
  # point. The default is fine unless you reduce map.resolution by a lot.
  neighbors_max_range = 5m
}

# What happens to a scan between arriving and being registered. Filters drop
# points before they cost anything downstream, so anything excluded here is work
# the registration never has to do.
preprocessing {
  # Reduces the number of points that reach the registration.
  downsample {
    # @recommended
    # Publish the full undistorted cloud on the output topic. This changes only
    # what is published. Registration always uses the downsampled cloud.
    bypass_output_downsample = true

    # Downsample the incoming scan by keeping only every n-th point. 1 means no points are skipped.
    point_skip = 1

    # Where the downsampling runs. Backend uses the active acceleration backend
    # when one is available, CPU always uses the CPU path.
    # CPU | Backend
    backend = Backend

    # Expected feature size in the incoming scan, as a factor of
    # registration.map.resolution. Values between 0.0 and 1.0 are expected.
    query_resolution_factor = 0.5
  }

  # How many scans at the very start are merged into one before registration
  # begins. A single scan from a non-repetitive scan pattern covers too little of
  # the surroundings to build a map from, so the platform has to stand still
  # until this many scans have arrived.
  initial_scan_accumulate = 3

  # Point filters. Each one is skipped while it is left at its default, and a
  # point has to pass all of them to reach the registration.
  filter {
    bounding_box {
      # Keeps points inside an axis-aligned box, given as one range per axis in
      # the LiDAR frame. An open end means no limit on that side, so the default
      # ".." on all three axes is a box of infinite size and keeps everything.

      # Range along x, for example 1m..3m.
      x = ..

      # Range along y, for example 2m.. for everything beyond 2 metres.
      y = ..

      # Range along z, for example 4cm..100m.
      z = ..

      # Keep the points outside the box instead of the ones inside it.
      invert = false
    }

    distance {
      # Keeps points whose distance from the sensor origin falls in this range.
      # The default drops everything closer than 4 metres, which is usually the
      # robot itself.

      # For example ..2m for everything up to 2 metres, or 1cm..4m for a band.
      range = 4m..

      # Keep the points outside the range instead of the ones inside it.
      invert = false
    }

    # Keeps points within a horizontal angular sector, measured around the
    # forward axis below. Useful where part of the field of view is permanently
    # blocked, for example by a mast or an arm.
    horizontal_fov = false
    horizontal_fov {
      # Angular range in degrees, for example 10..42.
      degrees = ..

      # Axis the angular range is measured from, as [x, y] in the LiDAR frame.
      forward = [1.0, 0.0]

      # Keep the points outside the sector instead of the ones inside it.
      invert = false
    }

    polygon_2d {
      # Keeps points inside an arbitrary polygon in the ground plane, for cases
      # a box or a sector cannot describe.

      # Vertices in order, for example [ [1.0m, -1.5m], [-1.0m, 1.5m], [0m, 0m] ].
      # An empty list disables the filter.
      coords = []

      # TF frame the coordinates are given in. When set, the polygon is
      # transformed into the sensor frame before it is applied, which makes it
      # possible to express a boundary that stays fixed in the world while the
      # robot moves. Empty means the coordinates are in the LiDAR frame.
      global_zero_frame = ""

      # Keep the points outside the polygon instead of the ones inside it.
      invert = false
    }

    intensity {
      # Keeps points whose intensity falls in this range. Useful for dropping
      # returns from retroreflective surfaces, which saturate and report a
      # geometry that is not there.

      # Range, normalized to 0.0..1.0.
      values = ..

      # Keep the points outside the range instead of the ones inside it.
      invert = false
    }
    
  }

}

# Devices Pelorus talks to directly instead of subscribing to a topic. These
# deliver both a point cloud and IMU measurements over the same connection, so
# they cannot live under `lidar` or `imu` alone. A driver here only starts when
# `lidar.driver.type` or `imu.driver.type` selects it, and requires a build with
# the matching feature.
sensor_drivers {
  ouster {
    # Sensor hostname or address used only to read metadata over HTTP. The
    # sensor must already be configured to send UDP data to this host.
    hostname = "os.local"

    # Optional Ouster metadata JSON file. Empty fetches live metadata from
    # hostname. A file is useful for deterministic replay and offline testing.
    metadata_path = ""

    # Local receive address and the sensor's configured UDP destination ports.
    bind_address = "0.0.0.0"
    lidar_port = 7502
    imu_port = 7503

    # Optional sender filter. Empty accepts Ouster packets from any address.
    device_ip = ""

    # Ouster points are transformed into the forward-left-up Sensor frame.
    lidar_frame_id = "os_sensor"
    imu_frame_id = "os_imu"

    timestamp {
      # Ptp | Host
      # Ptp requires TIME_FROM_PTP_1588 sensor configuration and validates sensor
      # timestamps against host UTC. Host preserves relative sensor timing while
      # anchoring each scan or IMU packet at receive time.
      mode = Ptp

      # Largest accepted difference between sensor UTC and host UTC in Ptp mode.
      max_host_skew = 20ms

      # How long to wait for usable packets before giving up. If the clock mode
      # above cannot be satisfied, Pelorus fails startup and says so.
      startup_timeout = 15s
    }
  }
}

# Where point clouds come from and what Pelorus may assume about them.
lidar {
  driver {
    # Disabled | Velodyne | Ouster | Robosense
    # Disabled keeps the existing external point-cloud input configured by lidar.topic.
    # Velodyne, Ouster and Robosense enable in-process UDP drivers and require Pelorus
    # to be built with their matching feature (or the aggregate `sensor-drivers` feature).
    type = Disabled

    velodyne {
      # UDP connection to a Velodyne sensor. Only used when type = Velodyne.

      # Vlp16 | Vlp32c | PuckLite | PuckHiRes
      # The configured model must match the product identifier in received packets.
      model = Vlp32c

      # Local IPv4 or IPv6 address on which to receive UDP packets. "0.0.0.0"
      # listens on every IPv4 interface.
      bind_address = "0.0.0.0"

      # UDP data-packet port. Must be in 1..=65535 and differ from position_port.
      data_port = 2368

      # UDP position/NMEA packet port. Must be in 1..=65535 and differ from data_port.
      position_port = 8308

      # Optional sender-address filter. Empty accepts packets from any device.
      # Otherwise this must be an IPv4 or IPv6 address, for example
      # "192.168.1.201".
      device_ip = ""

      # Frame assigned to native scans. Coordinates follow ROS REP-103:
      # +X forward, +Y left, +Z up.
      frame_id = "velodyne"

      # Revolution boundary in radians, measured in the sensor azimuth convention.
      # A scan is emitted whenever azimuth crosses this angle. Expected range is
      # 0.0 (inclusive) to 2*pi (exclusive). Ratslang currently has no angle quantity type.
      cut_angle = 0.0

      timestamp {
        # Sensor | Host
        # Sensor prefers UTC reconstructed from valid position-packet NMEA. If no
        # valid NMEA anchor is available and allow_host_hour_anchor is true, the
        # packet's microseconds-since-hour are combined with the nearest host UTC hour.
        # Host timestamps packets when they are received and stamps a scan from its
        # first packet. Host is useful when sensor UTC is unavailable, but includes
        # network and scheduling latency. Both modes keep per-point firing offsets.
        mode = Sensor

        # In Sensor mode, permit nearest-host-hour reconstruction when position/NMEA
        # UTC is unavailable. The host clock must be synchronized to the sensor's
        # time reference. This option has no effect in Host mode.
        allow_host_hour_anchor = true

        # Maximum accepted absolute difference between reconstructed sensor UTC and
        # host UTC. Must be positive. Used by Sensor mode. Host mode already uses
        # host time.
        max_host_skew = 20ms

        # Maximum positive duration to wait for usable packets or timestamps
        # during startup. If the configured clock mode cannot be satisfied,
        # Pelorus fails startup and says so.
        startup_timeout = 15s
      }
    }

    robosense {
      # UDP connection to a RoboSense solid-state sensor. Only used when
      # lidar.driver.type selects Robosense. The mechanical units (RS-16,
      # RS-32, Helios, Bpearl) speak a different packet format and are not
      # handled by this driver.

      # M1 | M2 | E1 | Mx
      # These differ in packet length and per-point layout, so a mismatch is
      # rejected on the first packet rather than decoded into noise.
      model = M1

      # Local IPv4 or IPv6 address on which to receive UDP packets. "0.0.0.0"
      # listens on every IPv4 interface.
      bind_address = "0.0.0.0"

      # UDP point-data (MSOP) port. Must be in 1..=65535 and differ from difop_port.
      msop_port = 6699

      # UDP device-info (DIFOP) port. Must be in 1..=65535 and differ from msop_port.
      # The driver reads the echo mode from it, which decides whether Mx reports
      # one return per channel or two. Geometry never depends on it: these
      # sensors put the direction of every point in the data packet itself.
      difop_port = 7788

      # Optional sender-address filter. Empty accepts packets from any device.
      # Otherwise this must be an IPv4 or IPv6 address, for example
      # "192.168.1.200".
      device_ip = ""

      # Frame assigned to native scans. Coordinates follow ROS REP-103:
      # +X forward, +Y left, +Z up.
      frame_id = "rslidar"

      timestamp {
        # Sensor | Host
        # Sensor uses the UTC stamp in each packet header and requires the sensor
        # to be time-synchronized, typically over PTP or GPS. Host stamps packets
        # when they are received, which includes network and scheduling latency.
        # Both modes keep per-point measurement offsets.
        mode = Sensor

        # Largest accepted difference between sensor UTC and host UTC in Sensor
        # mode. Must be positive. A sensor that is not synchronized at all fails
        # this immediately. One that is merely a few milliseconds out passes it
        # at startup and only fails later, so the driver also measures the offset
        # over the first seconds of a run and logs what it saw, warning when the
        # observed drift would use up this budget within a minute.
        max_host_skew = 20ms

        # Maximum positive duration to wait for usable packets during startup.
        # If the configured clock mode cannot be satisfied, Pelorus fails
        # startup and says so.
        startup_timeout = 15s
      }
    }
  }
  # @recommended
  # Topic carrying sensor_msgs/msg/PointCloud2. Ignored while lidar.driver.type
  # selects an in-process driver.
  topic = "/ouster/points"

  # PointCloud2 | Compressed
  # Selects the input message type when using mt_pubsub. Compressed requires the
  # default-enabled `compressed-pointcloud` Cargo feature. Other communication
  # backends always subscribe to PointCloud2.
  subscriber_mode = PointCloud2

  # @recommended
  # Vendor whose per-point timestamp layout the incoming cloud uses. Pelorus
  # needs this to deskew a scan, since the points of one revolution are measured
  # at different times. NoTimestamp treats the scan as if it were instantaneous,
  # which visibly smears geometry as soon as the platform moves.
  # Velodyne | Ouster | Livox | Robosense | LivoxMid360 | Hesai | Blickfeld | SickMultiScan | NoTimestamp
  vendor = NoTimestamp

  # @recommended
  # Reliable | BestEffort | SystemDefault
  # Anything other than Reliable drops messages under load and is not supported.
  qos.reliability = Reliable

  # How far the filter trusts the scan match against the map. Lower values pull
  # the estimate harder towards the registration result.
  covariance = 0.001

  # Use incoming LiDAR messages as a forced synchronization trigger. This costs
  # accuracy, but some surrounding libraries work better with it.
  as_sync = false
}


# Where IMU measurements come from and how far they can be trusted. This also
# holds the transform to the LiDAR. Both that transform and the noise figures
# are measurements, and the calibration section of the documentation covers how
# to take them.
imu {
  driver {
    # Disabled | Xsens | Ouster
    # Disabled keeps the existing external IMU input configured by imu.topic.
    # Xsens and Ouster enable in-process drivers and require Pelorus to be built with
    # their matching feature (or the aggregate `sensor-drivers` feature).
    type = Disabled

    xsens {
      # Serial connection to an Xsens MTi. Only used when type = Xsens.

      # Serial device path. A stable /dev/serial/by-id/... path is recommended over
      # /dev/ttyUSB0 because USB enumeration order can change between boots.
      port = "/dev/ttyUSB0"

      # Serial baud rate in bits per second. Must be positive and must match the device.
      baudrate = 115200

      # Frame assigned to native IMU measurements.
      frame_id = "imu"

      # Requested MTData2 output frequency in Hz when configure_device is true.
      # Must be in 1..=2000 and supported by the connected Xsens model and baud rate.
      output_rate = 400

      # Enu | Ned | Nwu
      # World-coordinate convention requested for orientation output:
      # Enu = East-North-Up, Ned = North-East-Down, Nwu = North-West-Up.
      # Acceleration and rate-of-turn remain expressed in the IMU body frame.
      coordinate_system = Enu

      # If true, put the device in configuration mode, request UTC, calibrated
      # acceleration, rate-of-turn, and quaternion MTData2 fields at output_rate,
      # then return it to measurement mode. If false, the device must already emit
      # all required fields in compatible precision and coordinates.
      configure_device = true

      timestamp {
        # Utc | Host
        # Utc requires a present and valid MTData2 UTC field and rejects measurements
        # whose clock differs from the host beyond max_host_skew. Host timestamps a
        # complete decoded frame at receipt time and is useful before GNSS/UTC becomes
        # valid, but includes serial transport and scheduling latency.
        mode = Utc

        # Maximum accepted absolute difference between Xsens UTC and host UTC.
        # Must be positive. Used by Utc mode. Host mode already uses host time.
        max_host_skew = 20ms

        # Maximum positive duration to wait for usable MTData2 input during
        # startup. If Utc cannot be satisfied, Pelorus fails startup and says so.
        startup_timeout = 15s
      }
    }
  }

  # @recommended
  # Topic carrying sensor_msgs/msg/Imu. Ignored while imu.driver.type selects an
  # in-process driver. This has to be the raw device output. Calibrating against
  # an orientation-filtered topic produces a wrong result.
  topic = "/ouster/imu"

  # @recommended
  # Reliable | BestEffort | SystemDefault
  # Anything other than Reliable drops messages under load and is not supported.
  qos.reliability = Reliable

  # How long to collect IMU measurements before the first scan is processed.
  # These samples give the filter its initial gravity direction, so the platform
  # has to stand still for at least this long after start-up.
  initial_setup = 100ms

  # @recommended
  from_lidar_transform {
    # Where the IMU sits relative to the LiDAR. Measure this. A value taken from
    # a drawing is often a few millimetres out, and a wrong lever arm shows up as
    # drift that scales with how hard the platform turns. See the calibration
    # section of the documentation.

    # Look the transform up in the TF tree instead of using the values below.
    # The lookup happens once, when the first IMU message arrives.
    use_tf_tree = false

    # Transform from the LiDAR frame into the IMU frame, so a point in LiDAR
    # coordinates becomes a point in IMU coordinates. `dedrunk extrinsics`
    # prints both values in this form, ready to paste.
    translation = [-6.253mm, 11.775mm, 28.535mm]
    rotation = [  [1.0, 0.0, 0.0],
                  [0.0, 1.0, 0.0],
                  [0.0, 0.0, 1.0] ]

    # Keep refining the extrinsic in the filter instead of holding it at the
    # values above, which then serve as the starting point.
    #
    # This is refinement, not calibration: the filter needs a starting guess
    # close enough to converge, and the extrinsic is only observable while the
    # platform is actually being excited. On a robot that mostly drives
    # straight it will drift along the unobservable directions and get worse.
    # Leave it off unless you know the motion excites all axes, and check the
    # result against what you started with.
    estimate_online = false
  }

  # Ignore the IMU entirely and run the odometry on the Lidar alone, with the
  # filter propagating at a constant twist between scans.
  #
  # This exists to produce a trajectory for calibrating the IMU against, which
  # is circular if the IMU helped produce it. It is not a navigation mode: with
  # no IMU the gravity direction is unobservable, so roll and pitch are
  # relative to wherever the sensor started and not to level, and there is
  # nothing to carry the estimate through a stretch where the geometry is
  # degenerate. The covariances below stop describing sensor noise and start
  # describing how much the constant-twist model can be trusted, so raise them.
  lidar_only = false

  # CreateOrientationOnly | FilterAccelerationAndCreateOrientation | FilterAccelerationOnly | Disable
  # Orientation filtering Pelorus applies to incoming IMU measurements. Leave it
  # at Disable when the driver or the device already filters, which is the usual
  # case. The result is published on output.imu_ahrs_filtered_topic and is never
  # fed back into the odometry.
  ahrs_filtering.method = Disable

  # Magnitude of gravity in m/s^2, used as the starting point for the filter.
  # 9.81 is close enough everywhere on Earth. The filter refines it from there.
  gravity = 9.81

  # How far the filter trusts the IMU between LiDAR corrections. Larger values
  # mean less trust, so the scan match pulls the estimate further on each update.
  #
  # The defaults are the FAST-LIO tuning values and are not a noise model of any
  # particular sensor. `calibration.parameter_file.use_noise` below replaces them
  # with figures measured from your own IMU.
  covariance_gyroscope = 0.1

  # Process noise on the gyroscope bias, i.e. how fast the filter lets that bias
  # wander.
  covariance_gyroscope_bias = 0.0001

  # How far the filter trusts the accelerometer between LiDAR corrections.
  covariance_acceleration = 0.1

  # Process noise on the accelerometer bias.
  covariance_acceleration_bias = 0.0001

  # Corrections applied to every IMU sample before the filter sees it.
  calibration {
    # Measure the gyroscope bias at start-up and subtract it. The platform has to
    # stand still while the first `samples` measurements arrive. Stored bias
    # figures drift with temperature and between power cycles, so a value
    # measured at start-up is more accurate than one read from a file.
    gyroscope = true

    # How many measurements the start-up bias estimate averages over.
    gyroscope.samples = 100

    # Kalibr-format IMU calibration to apply to every sample: scale factors,
    # axis misalignment and biases. Relative path "./<path>" from this config
    # file, or absolute "/<path>".
    #
    # Produced by https://github.com/stelzo/dedrunk, or by Kalibr itself with
    # --imu-models scale-misalignment. Record the bag for it with
    # `pelorus online raw imu intrinsics --output-rosbag <dir>`.
    parameter_file = ""

    # Take the covariances above from parameter_file instead of the
    # constants, deriving them from its measured noise densities and bias
    # random walks. This needs a file that carries the stochastic half too, so
    # one from `dedrunk merge`. `dedrunk intrinsics` alone does not produce one.
    #
    # Off by default because it moves numbers the odometry is tuned around.
    # Both the derived and the configured values are logged at startup, so the
    # ratio for this sensor is visible before you commit to it.
    parameter_file.use_noise = false

    # Seconds the IMU clock runs ahead of the Lidar clock, as reported by
    # `dedrunk extrinsics`. Pelorus subtracts it from every IMU timestamp, so
    # paste the number exactly as dedrunk prints it, sign included.
    #
    # This is a constant skew between the two sensors' clocks, not jitter. It
    # matters most while turning: at 1 rad/s, 10 ms of skew smears the scan by
    # about half a degree, which the registration then has to absorb.
    time_offset = 0s

    # Multiplier on the measured densities before they become covariances.
    #
    # Allan variance is measured on a completely still sensor at constant
    # temperature. A moving robot has vibration, thermal drift and timing
    # jitter that dominate it, so the honest noise level in operation is
    # higher than the one on the bench, and no amount of measuring will tell
    # you by how much. The Kalibr wiki suggests 10x or more for MEMS parts.
    noise_inflation = 10.0
  }

}

# An optional third source of motion. Wheel odometry constrains the estimate
# along the directions the platform can actually move, which helps most where the
# geometry is degenerate, such as in a long corridor.
wheel {
  # Topic carrying nav_msgs/msg/Odometry. Wheel odometry is experimental and has
  # not seen enough testing to be recommended, which is why the default topic
  # name is one nothing publishes on.
  topic = "/odom_experimental"

  # Reliable | BestEffort | SystemDefault
  # QoS the wheel odometry topic is subscribed with.
  qos.reliability = Reliable

  # Orientation of the wheel odometry frame relative to the IMU. The velocity
  # and its covariance stay in this frame until the filter evaluates the
  # observation. The reference-point offset is taken from the platform
  # base/Lidar/IMU calibration (or ackermann.imu_to_rear_axle).
  to_imu_transform {
    # Look the rotation up in the TF tree instead of using the value below. The
    # lookup happens once, when the first odometry message arrives.
    use_tf_tree = false

    # Rotation from the wheel odometry frame into the IMU frame.
    rotation = [ [1.0, 0.0, 0.0],
                 [0.0, 1.0, 0.0],
                 [0.0, 0.0, 1.0] ]
  }

  # How far the filter trusts the wheel odometry. Left unset, the covariance
  # carried in the Odometry message is used instead.
  # covariance = 0.001

  # Standard deviation of the kinematic constraints applied during undistortion.
  # Lower values constrain harder.
  kinematic_constraint_std = 0.001

  # @recommended
  # Full3D | Differential | Holonomic2D | Ackermann
  # Which directions of motion the platform is capable of. Anything other than
  # Full3D constrains the IEKF accordingly.
  kinematic_model = Full3D
  
  # @recommended
  # Offset from the IMU to the centre of the rear axle, in the IMU frame. Only
  # used when kinematic_model is Ackermann.
  ackermann.imu_to_rear_axle = [0.0m, 0.0m, 0.0m]
}

# What Pelorus publishes and where it publishes it. This covers the topics and
# their QoS, the frames poses are expressed in, the terminal interface and the
# Rerun visualization.
output {

  # @recommended
  # Lidar | ImuAndLidar
  # How often odometry is published. ImuAndLidar adds low-latency IMU predictions
  # between the LiDAR corrections, published at the IMU rate.
  odometry_rate = Lidar

  # PointCloud2 | Compressed
  # Selects the registered-cloud message type when using mt_pubsub. Compressed
  # requires the default-enabled `compressed-pointcloud` Cargo feature. The topic
  # name stays "registered" in either mode.
  publisher_mode = PointCloud2

  # What Pelorus prints while it runs.
  terminal {
    # @recommended
    # Show the interactive TUI with position, velocity, and performance metrics.
    # Set to false for non-interactive sessions.
    ui = true

    # Print timing metrics for each pipeline stage. This is what to turn on when
    # Pelorus is not keeping up with the sensor.
    debug = false

    # Refresh rate of the debug output, in frames per second.
    debug_fps = 1

    # Shortest interval at which a repeating warning is printed again, so that a
    # condition holding for a whole run does not fill the terminal.
    warning_debounce = 1s
     
    # @recommended
    # Trace | Debug | Info | Warn | Error
    # Everything at this level and above is logged.
    log_level = Warn
  }

  # @recommended
  # Frame the estimated poses and the TF messages are expressed in. It is placed
  # wherever the sensor was at start-up, so it is a local frame and carries no
  # relation to any map or world frame that already exists in your TF tree.
  global_frame = "lio_init"

  # @recommended
  base_link {
    # Frame the odometry and TF are published for. Empty publishes the pose of
    # the LiDAR itself. Set it to your robot's body frame to have Pelorus apply
    # the transform below and publish that pose.
    frame_id = ""

    from_lidar_transform {
      # Look the transform up in the TF tree instead of using the values below.
      # The lookup happens once, when the first sensor message arrives.
      use_tf_tree = false

      # Pose of the Lidar in the base_link frame.
      translation = [0.0mm, 0.0mm, 0.0mm]
      rotation = [ [1.0, 0.0, 0.0],
                   [0.0, 1.0, 0.0],
                   [0.0, 0.0, 1.0] ]
    }
  }

  # Live visualization. Pelorus sends to a Rerun viewer that is already running.
  # It never starts one itself. Requires a build with the `rerun` Cargo feature.
  rerun = false
  rerun {
    # Address of the already running Rerun server to stream visualizations to.
    server_addr = "127.0.0.1:9876"

    # Periodic snapshot of the whole registration map, colored by relevance score.
    # Off by default. Large maps can be expensive to render.
    map = false
    map {
      # Shortest interval between two snapshots.
      publish_interval = 1s

      # Size of the ellipsoid drawn per voxel, scaled by that voxel's covariance.
      visual_scale = 3.0
    }
  }

  # Reliable | BestEffort | SystemDefault
  # QoS of the LiDAR-rate odometry topic.
  lio_odometry_msg_qos = Reliable

  # Reliable | BestEffort | SystemDefault
  # QoS of the LiDAR-rate path topic. The path is a visualization aid, so
  # dropping messages under load is usually acceptable.
  lio_path_msg_qos = SystemDefault

  # Reliable | BestEffort | SystemDefault
  # QoS of the IMU-rate odometry topic, published when output.odometry_rate is
  # ImuAndLidar.
  imu_odometry_msg_qos = SystemDefault

  # Reliable | BestEffort | SystemDefault
  # QoS of the IMU-rate path topic.
  imu_path_msg_qos = SystemDefault

  # Reliable | BestEffort | SystemDefault
  # QoS of the AHRS-filtered IMU topic, published when imu.ahrs_filtering.method
  # is anything but Disable.
  imu_ahrs_filtered_msg_qos = Reliable

  # Topic the AHRS-filtered IMU output is published on, relative to
  # logger_prefix. Only used when imu.ahrs_filtering.method is not Disable.
  imu_ahrs_filtered_topic = "imu/filtered"

  # Namespace every published topic and every log line is prefixed with.
  logger_prefix = "pelorus"

  # Optional MCAP/ROS 2 bag recording of Pelorus output, unrelated to the live
  # pub/sub topics above. Off by default (both file and rosbag_dir empty).
  mcap {
    # Write a bare MCAP file at this path. Empty disables it. Mutually
    # exclusive with rosbag_dir.
    file = ""

    # Write a ROS 2 bag directory at this path instead of a bare MCAP file.
    # Always writes its own <rosbag_dir>/metadata.yaml. Empty disables it.
    # Mutually exclusive with file.
    rosbag_dir = ""

    # Also write a ROS 2 bag metadata.yaml next to `file`. Only valid together
    # with file (rosbag_dir already writes its own). Empty disables it.
    rosbag_metadata = ""

    # Topic names used inside the MCAP/ROS 2 bag. Unrelated to the live
    # pub/sub topics above.
    cloud_topic = "/registered"
    odom_topic = "/odom"
    path_topic = "/path"
    tf_topic = "/tf"
  }

  # Text files written while Pelorus runs. Empty writes nothing; "auto" names
  # the file after the current time, in the working directory.
  # TUM trajectory of the base link, one pose per scan (and per IMU sample
  # with odometry_rate = ImuAndLidar).
  trajectory = ""

  # Builds with the processing-time-export feature only. End-to-end latency
  # per scan, wall clock from its arrival until its registered cloud is
  # ready, as sensor timestamp and nanoseconds.
  latency = ""

  # Builds with the processing-time-export feature only. Time actually spent
  # computing each scan, without queueing and waiting.
  processing_time = ""
}

# How `pelorus offline` runs.
offline {
  # Stay up after the bag is processed, still publishing and serving, so a
  # viewer can keep showing the result. Exit with Ctrl-C.
  keep_open = false

  # Let Minot and Pelorus pub/sub use non-local networking, for example to
  # attach a viewer from another machine.
  open_router = false

  # Keep the blocks of a streamed Marina dataset and promote it locally.
  # False streams without retaining anything.
  cache_stream = true
}

# How much of the machine Pelorus is allowed to use. It also decides between the
# single-core and the multi-core implementation of each pipeline stage.
parallelism {
  # @recommended
  # Upper bound on the CPU threads used for parallel processing. `all` uses every
  # available core. Pelorus is meant to leave room for whatever else runs on the
  # machine, so raising this trades headroom for latency.
  num_threads = 3

  # Force CPU-only processing regardless of available GPU backends.
  cpu_only = false

  # @recommended
  # Auto | SingleCore | Rayon
  # How each pipeline stage picks between the single-core and the rayon
  # implementation. Auto measures both at runtime and keeps whichever wins. Pick
  # one of the other two when the execution path has to stay fixed, for example
  # when reporting runtime figures in a paper.
  #
  # num_threads = 1 still runs the rayon implementation with one thread. Setting
  # this to SingleCore is what actually takes rayon out of the path.
  execution_mode = Auto

  # Number of standard errors by which one CPU execution mode must beat the other
  # before Pelorus decides the execution mode for the next phase.
  decision_threshold_stddevs = 2.0

  # Number of measurements collected for each mode before a decision is allowed.
  decision_samples_per_mode = 20
}
