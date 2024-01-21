time spark-submit --executor-cores 4 \
    --executor-memory 12G \
    main.py > "out$(date '+%m%d%H%M').txt"
